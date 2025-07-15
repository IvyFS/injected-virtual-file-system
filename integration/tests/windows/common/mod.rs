use std::{ffi::OsStr, path::Path, process::Command};

use win_api::Win32::{
  Foundation::HANDLE,
  Storage::FileSystem::{GETFINALPATHNAMEBYHANDLE_FLAGS, GetFinalPathNameByHandleW},
};

pub const INJECTOR: &str = env!("CARGO_BIN_FILE_INJECTOR");
pub const WORKSPACE_ROOT: &str = env!("CARGO_WORKSPACE_DIR");

pub fn inject_self(virtual_root: impl AsRef<Path>, mount_point: impl AsRef<Path>) {
  let pid = format!("{}", std::process::id());
  let mut injector = Command::new(INJECTOR)
    // .stdout(Stdio::piped())
    // .stderr(Stdio::piped())
    .current_dir(WORKSPACE_ROOT)
    .args([
      &"existing" as &dyn AsRef<OsStr>,
      &"--pid",
      &pid,
      &"--virtual-root",
      &virtual_root.as_ref().as_os_str(),
      &"--mount-point",
      &mount_point.as_ref(),
    ])
    .spawn()
    .unwrap();

  assert!(injector.wait().unwrap().success());
}

pub fn workspace_root<'a>() -> &'a Path {
  Path::new(WORKSPACE_ROOT)
}

pub unsafe fn path_from_handle(handle: HANDLE) -> String {
  unsafe {
    const LEN: usize = 1024;
    let mut buffer = [0; LEN];
    let len = GetFinalPathNameByHandleW(handle, &mut buffer, Default::default());
    if len != 0 && len < LEN as u32 {
      String::from_utf16(&buffer[0..(len as usize)]).unwrap()
    } else {
      panic!("Returned path longer than buffer: {len}");
    }
  }
}
