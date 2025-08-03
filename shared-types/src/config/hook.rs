use serde::{Deserialize, Serialize};

use crate::{HookError, config::VirtualFsConfig};

pub use strum::IntoDiscriminant;

#[derive(Debug, Serialize, Deserialize)]
pub struct HookConfig {
  #[serde(rename = "sn")]
  pub socket_name: String,
  #[serde(rename = "lc")]
  pub logging_config: HookLoggingVariant,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, strum::FromRepr)]
pub enum HookLoggingVariant {
  Ipc,
  Stderr,
  None,
}
