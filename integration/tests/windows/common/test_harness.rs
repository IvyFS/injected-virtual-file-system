use std::{
  ffi::OsStr,
  path::{Path, PathBuf},
  process::Command,
  sync::{Mutex, MutexGuard},
};

use shared_types::config::{
  VirtualFsConfig,
  injector::{DebugConfig, InjectorConfig, TargetConfig},
};
use tempdir::TempDir;

use crate::common::INJECTOR;

static HARNESS_LOCK: Mutex<()> = Mutex::new(());

pub struct TestHarness {
  pub mount_dir: TempDir,
  pub mount_target: PathBuf,
  pub virtual_dir: TempDir,
  pub virtual_target: PathBuf,

  config_dir: TempDir,
  pub config_path: PathBuf,
  pub config: InjectorConfig,

  _global_test_lock: Option<MutexGuard<'static, ()>>,
  serial: bool,
}

impl TestHarness {
  pub fn new(executable: impl ToString) -> TestHarness {
    let mount_dir = TempDir::new("mount_dir").unwrap();
    let mount_target = mount_dir.path().join("expected");
    let virtual_dir = TempDir::new("virtual_dir").unwrap();
    let virtual_target = virtual_dir.path().join("expected");
    let config_dir = TempDir::new("config").unwrap();

    let config = InjectorConfig {
      virtual_filesystem: VirtualFsConfig {
        mount_point: mount_dir.path().to_owned(),
        virtual_root: virtual_dir.path().to_owned(),
      },
      debug: DebugConfig {
        print_hook_logs_to_console: true,
        ..DebugConfig::default()
      },
      target: TargetConfig {
        executable: executable.to_string(),
        working_dir: None,
        args: Vec::new(),
        pid: None,
      },
      exit_once_patched: false,
    };

    TestHarness {
      mount_dir,
      mount_target,
      virtual_dir,
      virtual_target,
      config_path: config_dir.path().join("config.toml"),
      config_dir,
      config,

      _global_test_lock: None,
      serial: true,
    }
  }

  pub fn with_args(mut self, args: impl IntoIterator<Item = String>) -> Self {
    self.set_args(args);
    self
  }

  pub fn set_args(&mut self, args: impl IntoIterator<Item = String>) -> &mut Self {
    self.config.target.args = args.into_iter().collect();
    self
  }

  pub fn lock(&mut self) {
    self._global_test_lock = Some(HARNESS_LOCK.lock().unwrap());
  }

  pub fn with_working_dir(mut self, working_dir: impl AsRef<Path>) -> Self {
    self.config.target.working_dir = Some(working_dir.as_ref().to_path_buf());
    self
  }

  pub fn expected_name(mut self, file_name: impl AsRef<OsStr>) -> Self {
    self.mount_target.set_file_name(&file_name);
    self.virtual_target.set_file_name(file_name);
    self
  }

  pub fn parallel(mut self) -> Self {
    self.serial = false;
    self
  }

  pub fn write_config(&mut self) -> &mut Self {
    let config_str = toml::to_string_pretty(&self.config).unwrap();
    std::fs::write(&self.config_path, config_str).unwrap();
    self
  }

  pub fn spawn_target(&mut self) {
    let child = Command::new(INJECTOR)
      .arg(self.config_path.as_os_str())
      .spawn()
      .unwrap();
    assert!(child.wait_with_output().unwrap().status.success());
  }

  pub fn write_config_and_output(&mut self) -> std::process::Output {
    self.write_config();
    self.spawn_output()
  }

  pub fn spawn_output(&mut self) -> std::process::Output {
    let output = Command::new(INJECTOR)
      .arg(self.config_path.as_os_str())
      .output()
      .unwrap();
    assert!(output.status.success());

    output
  }
}
