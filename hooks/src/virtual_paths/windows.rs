use std::path::{Path, PathBuf};

use shared_types::HookError;

use crate::windows::os_types::handles::{HANDLE_MAP, Handle, std_open_dir_handle_unhooked};

pub struct VirtualPath {
  pub path: PathBuf,
  pub original: PathBuf,
}

impl VirtualPath {
  pub fn open_all(path: impl AsRef<Path>, recreate: bool) -> Result<Handle, HookError> {
    fn recurse_open(path: &Path) -> Result<Handle, HookError> {
      if let Some(handle) = HANDLE_MAP.get_by_path(path) {
        Ok(handle.handle)
      } else {
        open_dir_cached(path)
      }
    }

    let path = path.as_ref();
    if recreate {
      recurse_open(
        path
          .parent()
          .expect("Path should have a parent/not be root"),
      )?;
      open_dir_cached(path)
    } else {
      recurse_open(path)
    }
  }
}

fn open_dir_cached(path: &Path) -> Result<Handle, HookError> {
  std_open_dir_handle_unhooked(path).inspect(|opened_handle| {
    HANDLE_MAP.insert(*opened_handle, path);
  })
}
