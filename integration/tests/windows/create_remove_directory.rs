use std::{path::Path, process::Command};

use shared_types::config::{
  VirtualFsConfig,
  injector::{DebugConfig, InjectorConfig, TargetConfig},
};
use tempdir::TempDir;

use crate::common::{INJECTOR, WORKSPACE_ROOT};

#[test]
fn create_directory_w() {
  // Folder we try to create a directory in
  let mount_dir = TempDir::new("mount_dir").unwrap();
  // Folder we'll try to create (and shouldn't exist)
  let target = mount_dir.path().join("expected");
  // Folder the directory will actually be created in
  let virtual_dir = TempDir::new("virtual_dir").unwrap();
  // Folder we should actually create
  let actual_target = virtual_dir.path().join("expected");

  if target.exists() {
    let _ = std::fs::remove_dir(&target);
  }
  if actual_target.exists() {
    let _ = std::fs::remove_dir(&actual_target);
  }

  let config_path = Path::new(WORKSPACE_ROOT).join("integration/assets/config.toml");
  let config = config(&mount_dir, &virtual_dir, &target);
  std::fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();

  let child = Command::new(INJECTOR)
    .arg(dbg!(config_path))
    .spawn()
    .unwrap();
  assert!(child.wait_with_output().unwrap().status.success());

  std::thread::sleep(std::time::Duration::from_secs(3));

  assert!(!target.exists());
  assert!(actual_target.exists());

  drop(mount_dir);
  drop(virtual_dir);
}

fn config(mount_dir: &TempDir, virtual_dir: &TempDir, target: &Path) -> InjectorConfig {
  InjectorConfig {
    virtual_filesystem: VirtualFsConfig {
      mount_point: mount_dir.path().to_owned(),
      virtual_root: virtual_dir.path().to_owned(),
    },
    debug: DebugConfig {
      print_hook_logs_to_console: true,
      suppress_target_output: false,
      ..Default::default()
    },
    target: TargetConfig {
      executable: env!("CARGO_BIN_EXE_DIR_EDIT").to_owned(),
      working_dir: None,
      args: vec![target.display().to_string()],
      pid: None,
    },
    exit_once_patched: true,
  }
}

#[test]
fn remove_directory_w() {
  // Folder we try to remove a directory from
  let mount_dir = TempDir::new("mount_dir").unwrap();
  // Folder we'll try to remove (and shouldn't exist)
  let target = mount_dir.path().join("expected");
  // Folder the directory will actually be removed from
  let virtual_dir = TempDir::new("virtual_dir").unwrap();
  // Folder we should actually remove
  let actual_target = virtual_dir.path().join("expected");

  if !target.exists() {
    std::fs::create_dir(&target).unwrap();
  }
  if !actual_target.exists() {
    std::fs::create_dir(&actual_target).unwrap();
  }

  let config_path = Path::new(WORKSPACE_ROOT).join("integration/assets/config.toml");
  let mut config = config(&mount_dir, &virtual_dir, &target);
  config.target.args.push("delete".to_owned());
  std::fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();

  let child = Command::new(INJECTOR)
    .arg(config_path.as_os_str())
    .spawn()
    .unwrap();
  assert!(child.wait_with_output().unwrap().status.success());

  std::thread::sleep(std::time::Duration::from_secs(3));

  assert!(target.exists());
  assert!(!actual_target.exists());

  drop(mount_dir);
  drop(virtual_dir);
}
