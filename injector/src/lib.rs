use std::{
  ffi::CString,
  io::{Cursor, LineWriter, Write},
  path::Path,
  process::exit,
  str::FromStr,
  sync::LazyLock,
  time::Duration,
};

use ::tracing::{info, trace};
use frida::{Device, Frida, Inject, OutputListener, SpawnOptions};
use shared_types::config::{
  hook::{HookConfig, HookLoggingVariant},
  injector::{InjectorConfig, TargetConfig},
};

use tracing::HOOKED_PROCESS_OUTPUT_TARGET;

use crate::{
  ipc::{PATCH_COMPLETE, generate_socket_name, start_message_listener},
  tracing::INJECTOR_PROFILING_TARGET,
};

pub mod config;
mod ipc;
mod tracing;

#[cfg(not(feature = "testing-no-embed"))]
static HOOK: &'static [u8] = include_bytes!(env!("CARGO_CDYLIB_FILE_AGENT"));
#[cfg(feature = "testing-no-embed")]
static HOOK: &str = env!("CARGO_CDYLIB_FILE_AGENT");

static FRIDA: LazyLock<Frida> = LazyLock::new(|| unsafe { Frida::obtain() });

pub async fn inject(config: InjectorConfig) {
  let target_config = config.target;

  let _guard = tracing::init_tracing(&config.debug);

  let device_manager = frida::DeviceManager::obtain(&FRIDA);
  let local_device = device_manager.get_local_device();

  let mut device = local_device.unwrap();

  trace!("[*] Frida version: {}", frida::Frida::version());
  trace!("[*] Device name: {}", device.get_name());

  let pid = target_config
    .pid
    .unwrap_or_else(|| spawn_target(&target_config, config.debug.pipe_target_output, &mut device));

  let (ns_name, socket_name) = generate_socket_name();
  start_message_listener(ns_name);

  let entry_data = HookConfig {
    socket_name,
    logging_config: match config.debug.enable_ipc_logging {
      true => HookLoggingVariant::Ipc,
      false if config.debug.print_hook_logs_to_console => HookLoggingVariant::Stderr,
      false => HookLoggingVariant::None,
    },
    fs_config: config.virtual_filesystem,
  }
  .encode()
  .unwrap();

  let patch_start = std::time::Instant::now();
  trace!(target: INJECTOR_PROFILING_TARGET, patch_start = format!("{patch_start:#?}"));
  #[cfg(not(feature = "testing-no-embed"))]
  let id = device
    .inject_library_blob_sync(pid, HOOK, "injected_function", entry_data)
    .unwrap();
  #[cfg(feature = "testing-no-embed")]
  let id = device
    .inject_library_file_sync(pid, HOOK, "injected_function", entry_data)
    .unwrap();

  trace!("*** Injected, id={id}");

  PATCH_COMPLETE.notified().await;
  trace!("*** Finished patching");
  let patch_end = std::time::Instant::now();
  let patch_total = patch_end - patch_start;
  trace!(target: INJECTOR_PROFILING_TARGET, patch_end = format!("{patch_end:#?}"), patch_total = format!("{patch_total:#?}"));

  // Don't wait if we're patching an already running process
  if target_config.pid.is_some() {
    exit(0)
  }

  let exit_code = await_process_or_ctrlc(&mut device, pid)
    .await
    .expect("Wait for child process termination or ctrl-c");

  if !config.instant_shutdown {
    let mut interval = tokio::time::interval(SHUTDOWN_INTERVAL);
    info!("Hooked process is dead. Shutting down IPC socket.");
    for count in 0..SHUTDOWN_COUNT {
      interval.tick().await;

      info!(
        "Terminating in {:.1}s",
        (SHUTDOWN_INTERVAL * (SHUTDOWN_COUNT - count) as u32).as_secs_f32()
      )
    }
  }
  info!("shutdown");

  if config.return_target_exit_code {
    exit(exit_code as i32)
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

async fn await_process_or_ctrlc(
  device: &mut Device<'_>,
  pid: u32,
) -> Result<u32, tokio::task::JoinError> {
  let await_process = await_process(pid);
  tokio::pin!(await_process);

  device.resume(pid).expect("Resume target");

  let ctrlc_signal = tokio::signal::ctrl_c();

  tokio::select! {
    exit_code = &mut await_process => {
      return exit_code;
    },
    _ = ctrlc_signal => {
      info!(target: "injector", "Got Ctrl-C - Killing hooked process");
      device.kill(pid).unwrap();
    }
  }
  await_process.await
}

#[cfg(windows)]
fn await_process(pid: u32) -> impl Future<Output = Result<u32, tokio::task::JoinError>> {
  use std::mem::MaybeUninit;

  use shared_types::unsafe_types::SendPtr;
  use win_api::Win32::{
    Foundation::HANDLE,
    System::Threading::{
      GetExitCodeProcess, INFINITE, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
      PROCESS_SYNCHRONIZE, WaitForSingleObject,
    },
  };

  let process = unsafe {
    OpenProcess(
      PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
      false,
      pid,
    )
  }
  .expect("Open handle to injected process");
  let process = SendPtr(process.0);

  tokio::task::spawn_blocking(move || {
    let _ = &process;
    let process = HANDLE(process.0);
    let res = unsafe { WaitForSingleObject(process, INFINITE) };

    if res.0 == 0 {
      let mut exit_code = MaybeUninit::uninit();
      unsafe {
        GetExitCodeProcess(process, exit_code.as_mut_ptr()).expect("Get process exit code");
        exit_code.assume_init()
      }
    } else {
      panic!(
        "Wait on process returned with unexpected event: {:x}",
        res.0
      )
    }
  })
}

#[cfg(not(windows))]
async fn await_process(pid: u32) -> Result<u32, tokio::task::JoinError> {
  use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, RefreshKind, System};

  let pid = Pid::from_u32(pid);
  let process_refresh = ProcessRefreshKind::nothing().without_tasks();
  let mut system =
    System::new_with_specifics(RefreshKind::nothing().with_processes(process_refresh));
  let mut interval = tokio::time::interval_at(
    (std::time::Instant::now() + Duration::from_secs(1)).into(),
    shared_types::DEFAULT_HEARTBEAT,
  );

  loop {
    interval.tick().await;

    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, process_refresh);

    if system.process(pid).is_none() {
      return Ok(0);
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
