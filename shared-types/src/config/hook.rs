use serde::{Deserialize, Serialize};

use crate::{HookError, config::VirtualFsConfig};

pub use strum::IntoDiscriminant;

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

#[derive(Debug, Serialize, Deserialize, strum::EnumDiscriminants)]
#[strum_discriminants(name(HookLoggingVariant), repr(u8), derive(strum::FromRepr))]
pub enum HookLoggingConfig {
  Ipc(String),
  Stderr,
  None,
}

#[cfg(test)]
mod test {
  use strum::IntoDiscriminant;

  use crate::config::hook::{HookLoggingConfig, HookLoggingVariant};

  #[test]
  fn hook_logging_config_repr_round_trip() {
    let logging_type = HookLoggingConfig::Ipc(String::new());
    let repr = logging_type.discriminant() as u8;
    assert_eq!(
      HookLoggingVariant::Ipc,
      HookLoggingVariant::from_repr(repr).unwrap()
    )
  }
}
