use std::{
  collections::HashMap,
  path::PathBuf,
  sync::{LazyLock, Mutex, MutexGuard, RwLock},
};

use shared_types::HookError;

#[cfg(windows)]
pub mod windows;

pub static MOUNT_POINT: LazyLock<RwLock<PathBuf>> = LazyLock::new(|| RwLock::new(PathBuf::new()));
pub static VIRTUAL_ROOT: LazyLock<RwLock<PathBuf>> = LazyLock::new(|| RwLock::new(PathBuf::new()));

static VIRTUAL_PATHS: LazyLock<Mutex<HashMap<PathBuf, PathBuf>>> =
  LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn get_virtual_paths_mut() -> Result<MutexGuard<'static, HashMap<PathBuf, PathBuf>>, HookError>
{
  Ok(VIRTUAL_PATHS.lock()?)
}
