use std::{
  path::{Path, PathBuf},
  process::Command,
  sync::{Mutex, MutexGuard},
};

use shared_types::config::{
  VirtualFsConfig,
  injector::{DebugConfig, InjectorConfig, TargetConfig},
};
use tempdir::TempDir;

use crate::INJECTOR;

static HARNESS_LOCK: Mutex<()> = Mutex::new(());

pub struct TestHarness {
  pub mount_dir: TempDir,
  mount_expected: PathBuf,
  pub virtual_dir: TempDir,
  virtual_expected: PathBuf,
  pub expected: PathBuf,
  pub extra_dir: TempDir,

  pub config_dir: TempDir,
  pub config_path: PathBuf,
  pub config: InjectorConfig,
  pub output_file: PathBuf,

  _global_test_lock: Option<MutexGuard<'static, ()>>,
  serial: bool,
}

impl TestHarness {
  pub fn new(executable: impl ToString) -> TestHarness {
    let expected = Path::new("expected").to_path_buf();
    let mount_dir = TempDir::new("mount_dir").unwrap();
    let mount_target = mount_dir.path().join(&expected);
    let virtual_dir = TempDir::new("virtual_dir").unwrap();
    let virtual_target = virtual_dir.path().join(&expected);
    let config_dir = TempDir::new("config").unwrap();

    let config = InjectorConfig {
      virtual_filesystem: VirtualFsConfig {
        mount_point: mount_dir.path().to_owned(),
        virtual_root: virtual_dir.path().to_owned(),
      },
      debug: DebugConfig {
        print_hook_logs_to_console: true,
        tracing_level: "OFF".parse().unwrap(),
        ..DebugConfig::default()
      },
      target: TargetConfig {
        executable: executable.to_string(),
        working_dir: None,
        args: Vec::new(),
        pid: None,
      },
      exit_once_patched: false,
      instant_shutdown: true,
    };

    TestHarness {
      mount_dir,
      mount_expected: mount_target,
      virtual_dir,
      virtual_expected: virtual_target,
      expected: expected.to_path_buf(),
      extra_dir: TempDir::new("extra").unwrap(),

      output_file: config_dir.path().join("output.json"),
      config_path: config_dir.path().join("config.toml"),
      config_dir,
      config,

      _global_test_lock: None,
      serial: true,
    }
  }

  pub fn with_args<T: ToString>(mut self, args: impl IntoIterator<Item = T>) -> Self {
    self.set_args(args);
    self
  }

  pub fn set_args<T: ToString>(&mut self, args: impl IntoIterator<Item = T>) -> &mut Self {
    self.config.target.args = args.into_iter().map(|arg| arg.to_string()).collect();
    self
  }

  pub fn lock(&mut self) {
    self._global_test_lock = Some(HARNESS_LOCK.lock().unwrap());
  }

  pub fn with_working_dir(mut self, working_dir: impl AsRef<Path>) -> Self {
    self.set_working_dir(working_dir);
    self
  }

  pub fn set_working_dir(&mut self, working_dir: impl AsRef<Path>) {
    self.config.target.working_dir = Some(working_dir.as_ref().to_path_buf());
  }

  pub fn expected_name(mut self, expected: impl AsRef<Path>) -> Self {
    self.expected = expected.as_ref().to_path_buf();
    self.mount_expected = self.mount_dir.path().join(&self.expected);
    self.virtual_expected = self.virtual_dir.path().join(&self.expected);
    self
  }

  pub fn mount_expected(&self) -> &Path {
    &self.mount_expected
  }

  pub fn virtual_expected(&self) -> &Path {
    &self.virtual_expected
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
