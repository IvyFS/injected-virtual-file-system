use std::sync::OnceLock;

use frida_gum::Gum;
use hooks::Patcher;
use shared_types::{config::hook::HookConfig, message::Message};

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
    HookConfig::decode(&data).unwrap()
  };

  static CELL: OnceLock<Gum> = OnceLock::new();
  let gum = CELL.get_or_init(Gum::obtain);

  // let process = Process::obtain(gum);
  // let modules = process.enumerate_modules();
  // let modules = modules.into_iter().fold(String::new(), |acc, module| {
  //   let module = format!("{}: {}", module.name(), module.path());
  //   format!("{acc}\n{module}")
  // });

  let patcher = Patcher::init(gum, &data.socket_name, data.logging_config, data.fs_config);
  if let Err(err) = patcher.patch_functions() {
    patcher.log(Message::Error(err.to_string()))
  }

  patcher.log(Message::FinishedPatching);
}
