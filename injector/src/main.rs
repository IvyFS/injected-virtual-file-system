use std::{
  ffi::CString,
  hash::{Hash, Hasher},
  io::{BufRead, BufReader},
  str::FromStr,
  sync::LazyLock,
};

use frida::{Frida, Inject, Injector, SpawnOptions};
use interprocess::local_socket::{
  GenericNamespaced, ListenerOptions, Stream, ToNsName, traits::ListenerExt,
};
use shared_types::{EntryData, Message, config::InjectorConfig};

static HOOK: &'static [u8] = include_bytes!(env!("DYLIB_PATH"));

static FRIDA: LazyLock<Frida> = LazyLock::new(|| unsafe { Frida::obtain() });

#[tokio::main]
async fn main() {
  let device_manager = frida::DeviceManager::obtain(&FRIDA);
  let local_device = device_manager.get_local_device();

  let config = InjectorConfig::from_args();
  let target_config = config.target;

  if let Ok(mut device) = local_device {
    println!("[*] Frida version: {}", frida::Frida::version());
    println!("[*] Device name: {}", device.get_name());

    let mut options = SpawnOptions::default().argv(target_config.args);

    if let Some(working_dir) = target_config.working_dir {
      options = options.cwd(&CString::from_str(&working_dir.to_string_lossy()).unwrap())
    }
    let pid = device.spawn(target_config.executable, &options).unwrap();

    let mut hasher = std::hash::DefaultHasher::new();
    std::process::id().hash(&mut hasher);
    let name = format!("{}.sock", hasher.finish());
    let ns_name = name.as_str().to_ns_name::<GenericNamespaced>().unwrap();
    let listener = ListenerOptions::new().name(ns_name).create_sync().unwrap();

    tokio::spawn(async move {
      for stream in listener.incoming().filter_map(Result::ok) {
        let mut reader = BufReader::new(stream);

        let has_remaining =
          |reader: &mut BufReader<Stream>| reader.fill_buf().map(|b| !b.is_empty());
        while let Ok(message) = Message::recv(&mut reader) {
          println!("{message}");

          if !has_remaining(&mut reader).unwrap() {
            break;
          }
        }
      }
    });

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
  }
}
