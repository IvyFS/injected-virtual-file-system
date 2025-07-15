use serde::{Deserialize, Serialize};

use crate::{HookError, config::VirtualFsConfig};

#[derive(Debug, Serialize, Deserialize)]
pub struct HookConfig {
  #[serde(rename = "lc")]
  pub logging_config: HookLoggingConfig,
  #[serde(rename = "fs")]
  pub fs_config: VirtualFsConfig,
}

impl HookConfig {
  pub fn encode(self) -> Result<Vec<u8>, HookError> {
    Ok(serde_json::to_string(&self)?.into())
  }

  pub fn decode(data: &str) -> Result<Self, HookError> {
    Ok(serde_json::from_str(data)?)
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HookLoggingConfig {
  Ipc(String),
  Stderr,
  None,
}
