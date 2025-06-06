use std::{
  ffi::CString,
  fs::read_to_string,
  hash::{Hash, Hasher},
  io::{BufRead, BufReader},
  str::FromStr,
  sync::LazyLock,
};

use frida::{Frida, Inject, Injector, SpawnOptions};
use interprocess::local_socket::{
  GenericNamespaced, ListenerOptions, Stream, ToNsName, traits::ListenerExt,
};
use serde::Deserialize;
use shared_types::{EntryData, Message};

static HOOK: &'static [u8] = include_bytes!(env!("DYLIB_PATH"));

static FRIDA: LazyLock<Frida> = LazyLock::new(|| unsafe { Frida::obtain() });

#[derive(Debug, Deserialize)]
struct Config {
  mount_point: String,

  executable: String,
  working_dir: Option<String>,
  args: Vec<String>,
}

#[tokio::main]
async fn main() {
  let device_manager = frida::DeviceManager::obtain(&FRIDA);
  let local_device = device_manager.get_local_device();

  let config_path = std::env::args().nth(1).unwrap();
  let config_str = read_to_string(config_path).unwrap();
  let config: Config = toml::from_str(&config_str).unwrap();

  // let args: Vec<String> = std::env::args().collect();
  // let program = &args[1];

  // if program.is_empty() {
  //   println!("No path given for executable");
  //   return;
  // }

  if let Ok(mut device) = local_device {
    println!("[*] Frida version: {}", frida::Frida::version());
    println!("[*] Device name: {}", device.get_name());

    let mut options = SpawnOptions::default().argv(config.args);

    if let Some(working_dir) = config.working_dir {
      options = options.cwd(CString::from_str(&working_dir).unwrap())
    }
    // if args.len() >= 3 {
    //   if let [.., target_args] = &args[..] {
    //     options = options.argv(target_args.split(" "))
    //   }
    // }
    // if args.len() >= 4 && !&args[2].is_empty() {
    //   options = options.cwd(CString::new(args[2].bytes().collect::<Vec<_>>()).unwrap())
    // }
    let pid = device.spawn(config.executable, &options).unwrap();

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
      mount_point: config.mount_point,
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
