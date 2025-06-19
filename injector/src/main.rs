use std::{
  ffi::CString,
  hash::{Hash, Hasher},
  io::{BufRead, BufReader, Cursor, LineWriter, Write},
  str::FromStr,
  sync::{LazyLock, Mutex},
  time::Duration,
};

use frida::{Frida, Inject, Injector, OutputListener, SpawnOptions};
use interprocess::local_socket::{
  GenericNamespaced, Listener, ListenerOptions, Name, Stream, ToNsName,
  traits::{ListenerExt, Stream as _},
};
use shared_types::{EntryData, Message, config::InjectorConfig};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

static HOOK: &'static [u8] = include_bytes!(env!("DYLIB_PATH"));

static FRIDA: LazyLock<Frida> = LazyLock::new(|| unsafe { Frida::obtain() });

fn main() {
  let config = InjectorConfig::from_args();
  let target_config = config.target;

  let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
  tracing_subscriber::fmt()
    .with_writer(non_blocking)
    .with_env_filter(
      EnvFilter::builder()
        .with_default_directive(config.debug.tracing_level.into())
        .from_env_lossy(),
    )
    .init();

  let device_manager = frida::DeviceManager::obtain(&FRIDA);
  let local_device = device_manager.get_local_device();

  if let Ok(mut device) = local_device {
    println!("[*] Frida version: {}", frida::Frida::version());
    println!("[*] Device name: {}", device.get_name());

    let mut options = SpawnOptions::default()
      .stdio(frida::SpawnStdio::Pipe)
      .argv(target_config.args);

    if let Some(working_dir) = target_config.working_dir {
      options = options.cwd(&CString::from_str(&working_dir.to_string_lossy()).unwrap())
    }

    device.add_output_listener(Output);

    let pid = device.spawn(target_config.executable, &options).unwrap();

    let (runtime, names) = if dbg!(config.debug.enable_hook_logging) {
      let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

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

      runtime.spawn_blocking(|| message_listener(listener));

      Some((runtime, (name, ns_name)))
    } else {
      None
    }
    .unzip();
    let (name, ns_name) = names.unzip();

    let entry_data = EntryData {
      socket_name: name,
      fs_config: config.virtual_filesystem,
    }
    .encode()
    .unwrap();

    let id = Injector::new()
      .inject_library_blob_sync(pid, HOOK, "injected_function", entry_data)
      .unwrap();

    println!("*** Injected, id={id}");

    device.resume(pid).unwrap();

    if let Some((runtime, ns_name)) = runtime.zip(ns_name) {
      runtime.block_on(deadline_check(pid, ns_name));
    }
  }
}

fn message_listener(listener: Listener) {
  for stream in listener.incoming().filter_map(Result::ok) {
    let mut reader = BufReader::new(stream);

    let has_remaining = |reader: &mut BufReader<Stream>| reader.fill_buf().map(|b| !b.is_empty());
    while let Ok(message) = Message::recv(&mut reader) {
      match message {
        Message::ShutdownCountdown(count) => info!(
          "Terminating in {:.1}s",
          (SHUTDOWN_INTERVAL * (SHUTDOWN_COUNT - count) as u32).as_secs_f32()
        ),
        Message::ShutdownFinal => {
          info!("shutdown");
          return;
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

impl OutputListener for Output {
  fn on_output(pid: u32, fd: i8, data: Vec<u8>) {
    static BUFFER: LazyLock<Mutex<LineWriter<Cursor<Vec<u8>>>>> =
      LazyLock::new(|| Mutex::new(LineWriter::new(Default::default())));

    let mut buf = BUFFER.lock().unwrap();
    buf.write_all(&data).unwrap();

    let inner = buf.get_mut();
    if inner.position() > 0 {
      let line = String::from_utf8_lossy(inner.get_ref());
      line
        .split_terminator('\n')
        .for_each(|line| info!(target: "hooked_process.stdout", pid, fd, "{line}"));
      inner.get_mut().clear();
      inner.set_position(0);
    }
  }
}
