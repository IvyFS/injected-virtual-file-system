use std::{fs::read_to_string, path::PathBuf};

use serde::{Deserialize, Serialize};
use tracing::level_filters::LevelFilter;

#[derive(Debug, Deserialize)]
pub struct InjectorConfig {
  pub virtual_filesystem: VirtualFsConfig,
  #[serde(default)]
  pub debug: DebugConfig,
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
  #[serde(default)]
  pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VirtualFsConfig {
  #[serde(alias = "mp", rename(serialize = "mp"))]
  pub mount_point: PathBuf,
  #[serde(alias = "vr", rename(serialize = "vr"))]
  pub virtual_root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DebugConfig {
  #[serde(default = "_true")]
  pub enable_hook_logging: bool,
  #[serde(with = "filter_serde", default = "_level")]
  pub tracing_level: LevelFilter,
}

const fn _true() -> bool {
  true
}

const fn _level() -> LevelFilter {
  LevelFilter::INFO
}

impl Default for DebugConfig {
  fn default() -> Self {
    Self {
      enable_hook_logging: Default::default(),
      tracing_level: LevelFilter::INFO,
    }
  }
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
