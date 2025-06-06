use std::sync::OnceLock;

use frida_gum::{Gum, Process};
use hooks::Patcher;
use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName, traits::Stream as _};
use shared_types::{EntryData, Message};

#[unsafe(no_mangle)]
unsafe fn injected_function(data: *const std::os::raw::c_char, stay_resident: *mut u32) {
  let data = unsafe {
    *stay_resident = 1;

    let Some(ptr) = data.as_ref() else {
      // println!("Didn't get data");
      return;
    };
    let data = std::ffi::CStr::from_ptr(ptr).to_string_lossy();
    // println!("injected_function called with data: '{socket_name}'");
    EntryData::decode(&data).unwrap()
  };

  static CELL: OnceLock<Gum> = OnceLock::new();
  let gum = CELL.get_or_init(Gum::obtain);

  let ns_name = data.socket_name.to_ns_name::<GenericNamespaced>().unwrap();
  let stream = Stream::connect(ns_name).unwrap();

  let process = Process::obtain(gum);
  let modules = process.enumerate_modules();
  let modules = modules.into_iter().fold(String::new(), |acc, module| {
    let module = format!("{}: {}", module.name(), module.path());
    format!("{acc}\n{module}")
  });

  let patcher = Patcher::init(gum, stream, data.mount_point);
  if let Err(err) = patcher.patch_functions() {
    patcher.log(Message::Error(err.to_string()))
  }

  patcher.log(Message::DebugGetModules(modules));
  patcher.log(Message::FinishedPatching);
}
