use std::{
  path::PathBuf,
  sync::{LazyLock, RwLock},
};

#[cfg(windows)]
pub mod windows;

pub static MOUNT_POINT: LazyLock<RwLock<PathBuf>> = LazyLock::new(|| RwLock::new(PathBuf::new()));
pub static VIRTUAL_ROOT: LazyLock<RwLock<PathBuf>> = LazyLock::new(|| RwLock::new(PathBuf::new()));
