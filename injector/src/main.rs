use std::{
  ffi::CString,
  io::{Cursor, LineWriter, Write},
  path::Path,
  process::exit,
  str::FromStr,
  sync::LazyLock,
  time::{Duration, Instant},
};

use ::tracing::{info, trace};
use clap::Parser;
use frida::{Device, Frida, Inject, OutputListener, SpawnOptions};
use shared_types::config::{
  hook::{HookConfig, HookLoggingConfig},
  injector::{InjectorConfig, TargetConfig},
};

use config::Cli;
use tracing::HOOKED_PROCESS_OUTPUT_TARGET;

use crate::ipc::{PATCH_COMPLETE, generate_socket_name, start_message_listener};

mod config;
mod ipc;
mod tracing;

#[cfg(not(feature = "testing-no-embed"))]
static HOOK: &'static [u8] = include_bytes!(env!("CARGO_CDYLIB_FILE_AGENT"));
#[cfg(feature = "testing-no-embed")]
static HOOK: &str = env!("CARGO_CDYLIB_FILE_AGENT");

static FRIDA: LazyLock<Frida> = LazyLock::new(|| unsafe { Frida::obtain() });

#[tokio::main]
async fn main() {
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

  let (ns_name, socket_name) = generate_socket_name();
  start_message_listener(
    ns_name,
    exit_once_patched || !config.debug.enable_ipc_logging,
  );

  let entry_data = HookConfig {
    logging_config: match config.debug.enable_ipc_logging {
      true => HookLoggingConfig::Ipc(socket_name),
      false if config.debug.print_hook_logs_to_console => HookLoggingConfig::Stderr,
      false => HookLoggingConfig::None,
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
    PATCH_COMPLETE.notified().await;
    trace!("*** Finished patching");
    device.resume(pid).unwrap();
  }

  if !exit_once_patched {
    let ctrlc_signal = tokio::signal::ctrl_c();
    let deadline_check = deadline_check(pid);
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
  } else {
    trace!("Exiting");
    exit(0)
  }
}

fn spawn_target(target_config: &TargetConfig, pipe_stdio: bool, device: &mut Device) -> u32 {
  let executable_name = Path::new(&target_config.executable)
    .file_name()
    .map(|file_name| file_name.to_str())
    .flatten()
    .unwrap_or_default();
  let args =
    std::iter::once(executable_name).chain(target_config.args.iter().map(|arg| arg.as_str()));
  let mut options = SpawnOptions::default().argv(args);

  if let Some(working_dir) = &target_config.working_dir {
    options = options.cwd(CString::from_str(&working_dir.to_string_lossy()).unwrap())
  }

  if pipe_stdio {
    options = options.stdio(frida::SpawnStdio::Pipe);
    device.add_output_listener(Output);
  }

  device.spawn(&target_config.executable, &options).unwrap()
}

const SHUTDOWN_COUNT: usize = 3;
const SHUTDOWN_INTERVAL: Duration = Duration::from_secs(1);

async fn deadline_check(pid: u32) {
  use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

  let pid = Pid::from_u32(pid);
  let process_refresh = ProcessRefreshKind::nothing().without_tasks();
  let mut system =
    System::new_with_specifics(RefreshKind::nothing().with_processes(process_refresh));
  let mut interval = tokio::time::interval_at(
    (Instant::now() + Duration::from_secs(1)).into(),
    shared_types::DEFAULT_HEARTBEAT,
  );

  loop {
    interval.tick().await;

    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, process_refresh);
    if system.process(pid).filter(|proc| proc.exists()).is_none() {
      let mut interval = tokio::time::interval(SHUTDOWN_INTERVAL);
      info!("Hooked process is dead. Shutting down IPC socket.");
      for count in 0..SHUTDOWN_COUNT {
        interval.tick().await;

        info!(
          "Terminating in {:.1}s",
          (SHUTDOWN_INTERVAL * (SHUTDOWN_COUNT - count) as u32).as_secs_f32()
        )
      }
      info!("shutdown");
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
