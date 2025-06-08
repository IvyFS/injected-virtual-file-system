use std::{
  collections::HashMap,
  path::PathBuf,
  sync::{LazyLock, Mutex, MutexGuard},
};

use shared_types::HookError;

pub static MOUNT_POINT: LazyLock<Mutex<PathBuf>> = LazyLock::new(|| Mutex::new(PathBuf::new()));

static VIRTUAL_PATHS: LazyLock<Mutex<HashMap<PathBuf, PathBuf>>> =
  LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn get_virtual_paths_mut() -> Result<MutexGuard<'static, HashMap<PathBuf, PathBuf>>, HookError>
{
  Ok(VIRTUAL_PATHS.lock()?)
}
