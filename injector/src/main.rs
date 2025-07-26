use std::{
  ffi::CString,
  hash::{Hash, Hasher},
  io::{BufRead, BufReader, Cursor, LineWriter, Write},
  process::exit,
  str::FromStr,
  sync::LazyLock,
  time::Duration,
};

use ::tracing::{debug, info, trace};
use clap::Parser;
use frida::{Device, Frida, Inject, OutputListener, SpawnOptions};
use interprocess::local_socket::{
  GenericNamespaced, Listener, ListenerOptions, Name, Stream, ToNsName,
  traits::{ListenerExt, Stream as _},
};
use shared_types::{
  Message,
  config::{
    hook::{HookConfig, HookLoggingConfig},
    injector::{InjectorConfig, TargetConfig},
  },
};
use tokio::sync::Notify;

use config::Cli;
use tracing::HOOKED_PROCESS_OUTPUT_TARGET;

mod config;
mod tracing;

#[cfg(not(feature = "testing-no-embed"))]
static HOOK: &'static [u8] = include_bytes!(env!("CARGO_CDYLIB_FILE_AGENT"));
#[cfg(feature = "testing-no-embed")]
static HOOK: &str = env!("CARGO_CDYLIB_FILE_AGENT");

static FRIDA: LazyLock<Frida> = LazyLock::new(|| unsafe { Frida::obtain() });

static NOTIFIER: Notify = Notify::const_new();

fn main() {
  let cli = Cli::parse();
  let config = InjectorConfig::from(cli);

  let target_config = config.target;
  let exit_once_patched = config.exit_once_patched;

  let _guard = tracing::init_tracing(&config.debug, exit_once_patched);

  let device_manager = frida::DeviceManager::obtain(&FRIDA);
  let local_device = device_manager.get_local_device();

  let mut device = local_device.unwrap();

  trace!("[*] Frida version: {}", frida::Frida::version());
  trace!("[*] Device name: {}", device.get_name());

  let pid = target_config
    .pid
    .unwrap_or_else(|| spawn_target(&target_config, config.debug.pipe_target_output, &mut device));

  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .unwrap();

  let (name, ns_name) = if config.debug.enable_ipc_logging {
    let mut hasher = std::hash::DefaultHasher::new();
    std::process::id().hash(&mut hasher);
    let name = format!("{}.sock", hasher.finish());
    let ns_name = name
      .as_str()
      .to_ns_name::<GenericNamespaced>()
      .unwrap()
      .into_owned();
    let listener = ListenerOptions::new()
      .name(ns_name.borrow())
      .create_sync()
      .unwrap();

    runtime.spawn_blocking(move || message_listener(listener, exit_once_patched));

    Some((name, ns_name))
  } else {
    None
  }
  .unzip();

  let entry_data = HookConfig {
    logging_config: match name {
      Some(ipc_name) => HookLoggingConfig::Ipc(ipc_name),
      None if config.debug.print_hook_logs_to_console => HookLoggingConfig::Stderr,
      None => HookLoggingConfig::None,
    },
    fs_config: config.virtual_filesystem,
  }
  .encode()
  .unwrap();

  #[cfg(not(feature = "testing-no-embed"))]
  let id = device
    .inject_library_blob_sync(pid, HOOK, "injected_function", entry_data)
    .unwrap();
  #[cfg(feature = "testing-no-embed")]
  let id = device
    .inject_library_file_sync(pid, HOOK, "injected_function", entry_data)
    .unwrap();

  trace!("*** Injected, id={id}");

  if target_config.pid.is_none() {
    device.resume(pid).unwrap();
  }

  if !exit_once_patched {
    let deadline_check = async move {
      if let Some(ns_name) = ns_name {
        deadline_check(pid, ns_name).await;
      }
    };
    let ctrlc_signal = tokio::signal::ctrl_c();
    runtime.block_on(async move {
      // NOTIFIER.notified().await;

      tokio::pin!(deadline_check);

      tokio::select! {
        _ = &mut deadline_check => {
          return;
        },
        _ = ctrlc_signal => {
          info!(target: "injector", "Got Ctrl-C - Killing hooked process");
          device.kill(pid).unwrap();
        }
      }
      deadline_check.await;
    });
  } else {
    exit(0)
  }
}

fn spawn_target(target_config: &TargetConfig, pipe_stdio: bool, device: &mut Device) -> u32 {
  let mut options = SpawnOptions::default().argv(&target_config.args);

  if let Some(working_dir) = &target_config.working_dir {
    options = options.cwd(CString::from_str(&working_dir.to_string_lossy()).unwrap())
  }

  if pipe_stdio {
    options = options.stdio(frida::SpawnStdio::Pipe);
    device.add_output_listener(Output);
  }

  device.spawn(&target_config.executable, &options).unwrap()
}

// TODO: split this into a public module
fn message_listener(listener: Listener, exit_once_patched: bool) {
  for stream in listener.incoming().filter_map(Result::ok) {
    let mut reader = BufReader::new(stream);

    let has_remaining = |reader: &mut BufReader<Stream>| reader.fill_buf().map(|b| !b.is_empty());
    let mut shutdown_started = false;
    while let Ok(message) = Message::recv(&mut reader) {
      match message {
        Message::ShutdownCountdown(count) => {
          if !shutdown_started {
            info!("Hooked process is dead. Shutting down IPC socket.");
            shutdown_started = true;
          }
          info!(
            "Terminating in {:.1}s",
            (SHUTDOWN_INTERVAL * (SHUTDOWN_COUNT - count) as u32).as_secs_f32()
          )
        }
        Message::ShutdownFinal => {
          info!("shutdown");
          return;
        }
        Message::FinishedPatching => {
          NOTIFIER.notify_one();
          if exit_once_patched {
            println!("exiting early");
            return;
          }
        }
        _ => {
          debug!(target: "hooked_process.hooks", "{message}")
        }
      }

      if !has_remaining(&mut reader).unwrap() {
        break;
      }
    }
  }
}

const SHUTDOWN_COUNT: usize = 3;
const SHUTDOWN_INTERVAL: Duration = Duration::from_secs(1);

async fn deadline_check(pid: u32, ns_name: Name<'_>) {
  use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

  let pid = Pid::from_u32(pid);
  let process_refresh = ProcessRefreshKind::nothing().without_tasks();
  let mut system =
    System::new_with_specifics(RefreshKind::nothing().with_processes(process_refresh));
  loop {
    tokio::time::sleep(shared_types::DEFAULT_HEARTBEAT).await;

    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, process_refresh);
    if system.process(pid).filter(|proc| !proc.exists()).is_none() {
      let mut shutdown_sender = Stream::connect(ns_name).unwrap();
      let mut interval = tokio::time::interval(SHUTDOWN_INTERVAL);
      for count in 0..SHUTDOWN_COUNT {
        interval.tick().await;
        Message::ShutdownCountdown(count)
          .send(&mut shutdown_sender)
          .unwrap();
      }
      Message::ShutdownFinal.send(&mut shutdown_sender).unwrap();
      return;
    }
  }
}

struct Output;

// TODO: implement listener cleanup
impl OutputListener for Output {
  fn on_output(_pid: u32, _fd: i8, _data: Vec<u8>) {
    unimplemented!()
  }

  fn on_output_with_context(
    pid: u32,
    fd: i8,
    data: Vec<u8>,
    buf: &mut LineWriter<Cursor<Vec<u8>>>,
  ) {
    buf.write_all(&data).unwrap();

    let inner = buf.get_mut();
    if inner.position() > 0 {
      let line = String::from_utf8_lossy(inner.get_ref());
      line.split_terminator('\n').for_each(|line| {
        info!(target: HOOKED_PROCESS_OUTPUT_TARGET, pid, fd);
        info!(target: HOOKED_PROCESS_OUTPUT_TARGET, "{line}");
        info!(target: HOOKED_PROCESS_OUTPUT_TARGET, "---");
      });
      inner.get_mut().clear();
      inner.set_position(0);
    }
  }
}
