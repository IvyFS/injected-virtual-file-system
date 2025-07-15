use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub mod hook;
pub mod injector;

#[derive(Debug, Serialize, Deserialize)]
pub struct VirtualFsConfig {
  #[serde(alias = "mp", rename(serialize = "mp"))]
  pub mount_point: PathBuf,
  #[serde(alias = "vr", rename(serialize = "vr"))]
  pub virtual_root: PathBuf,
}
