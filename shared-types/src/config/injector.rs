use std::{fs::read_to_string, path::PathBuf};

use serde::{Deserialize, Serialize};
use tracing::level_filters::LevelFilter;

use crate::config::VirtualFsConfig;

#[derive(Debug, Deserialize)]
pub struct InjectorConfig {
  pub virtual_filesystem: VirtualFsConfig,
  #[serde(default)]
  pub debug: DebugConfig,
  pub target: TargetConfig,
  #[serde(default)]
  pub exit_once_patched: bool,
}

impl InjectorConfig {
  pub fn parse_or_panic(path: impl AsRef<std::path::Path>) -> Self {
    let config_str = read_to_string(path).unwrap();
    toml::from_str(&config_str).unwrap()
  }
}

#[derive(Debug, Deserialize, Default)]
pub struct TargetConfig {
  pub executable: String,
  pub working_dir: Option<PathBuf>,
  #[serde(default)]
  pub args: Vec<String>,

  #[serde(skip)]
  pub pid: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DebugConfig {
  #[serde(default = "_true")]
  pub enable_ipc_logging: bool,
  #[serde(with = "filter_serde", default = "_level")]
  pub tracing_level: LevelFilter,
  #[serde(default)]
  pub suppress_target_output: bool,
  #[serde(default)]
  pub print_hook_logs_to_console: bool,
  #[serde(default = "_true")]
  pub pipe_target_output: bool,
}

impl Default for DebugConfig {
  fn default() -> Self {
    Self {
      enable_ipc_logging: false,
      tracing_level: LevelFilter::INFO,
      suppress_target_output: false,
      print_hook_logs_to_console: false,
      pipe_target_output: false
    }
  }
}

const fn _true() -> bool {
  true
}

const fn _level() -> LevelFilter {
  LevelFilter::INFO
}

mod filter_serde {
  use std::str::FromStr;

  use serde::{Deserialize, Deserializer, Serializer};
  use tracing::level_filters::LevelFilter;

  pub fn serialize<S: Serializer>(filter: &LevelFilter, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_str(&filter.to_string())
  }

  pub fn deserialize<'de, D: Deserializer<'de>>(deser: D) -> Result<LevelFilter, D::Error> {
    let str = String::deserialize(deser)?;
    LevelFilter::from_str(&str).map_err(serde::de::Error::custom)
  }
}
