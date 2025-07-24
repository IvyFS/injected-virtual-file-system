use std::path::Path;

use shared_types::HookError;

use crate::virtual_paths::{MOUNT_POINT, VIRTUAL_ROOT, windows::VirtualPath};

pub const NT_PATH_PREFIX: &str = "\\??\\";

pub fn strip_nt_prefix(path: &impl AsRef<Path>) -> &Path {
  let path = path.as_ref();
  path.strip_prefix(NT_PATH_PREFIX).unwrap_or(path)
}

pub fn get_virtual_path(path: impl AsRef<Path>) -> Result<Option<VirtualPath>, HookError> {
  let trimmed = strip_nt_prefix(&path);
  let canon = dunce::simplified(trimmed).to_path_buf();

  match canon.strip_prefix(MOUNT_POINT.read()?.as_path()) {
    Ok(stem) => {
      let virtual_root = VIRTUAL_ROOT.read()?;
      let rerouted_path = if !stem.as_os_str().is_empty() {
        virtual_root.join(stem)
      } else {
        virtual_root.to_path_buf()
      };
      Ok(Some(VirtualPath {
        path: rerouted_path,
        original: canon.to_path_buf(),
      }))
    }
    _ => Ok(None),
  }
}
