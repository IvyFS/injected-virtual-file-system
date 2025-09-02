use std::{ffi::OsStr, path::Path, process::Command, sync::Once};

pub mod output;
#[allow(dead_code)]
pub mod test_harness;

pub use test_harness::TestHarness;

pub const INJECTOR: &str = env!("CARGO_BIN_FILE_INJECTOR");
pub const WORKSPACE_ROOT: &str = env!("CARGO_WORKSPACE_DIR");

static PATCHED: Once = Once::new();

pub fn inject_self(virtual_root: impl AsRef<Path>, mount_point: impl AsRef<Path>) {
  PATCHED.call_once_force(|_| {
    let pid = format!("{}", std::process::id());
    let mut injector = Command::new(INJECTOR)
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
  });
}

pub fn workspace_root<'a>() -> &'a Path {
  Path::new(WORKSPACE_ROOT)
}

#[ext_trait::extension(pub trait PrcoessOutputExt)]
impl std::process::Output {
  fn fmt_stdio(&self) -> String {
    format!(
      "stdout: {}\nstderr: {}",
      String::from_utf8_lossy(&self.stdout),
      String::from_utf8_lossy(&self.stderr)
    )
  }
}
