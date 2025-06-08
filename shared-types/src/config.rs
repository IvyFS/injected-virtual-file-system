use std::{fs::read_to_string, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct InjectorConfig {
  pub virtual_filesystem: VirtualFsConfig,

  pub target: TargetConfig,
}

impl InjectorConfig {
  pub fn from_args() -> Self {
    let config_path = std::env::args().nth(1).unwrap();
    let config_str = read_to_string(config_path).unwrap();
    toml::from_str(&config_str).unwrap()
  }
}

#[derive(Debug, Deserialize)]
pub struct TargetConfig {
  pub executable: String,
  pub working_dir: Option<PathBuf>,
  pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VirtualFsConfig {
  pub mount_point: PathBuf
}
