use proc_macros::patch_fn;
use win_api::Win32::Security::SECURITY_ATTRIBUTES;
use win_types::{BOOL, PCWSTR};

use crate::{
  log::trace_expr,
  virtual_paths::windows::{VirtualPathOption, get_virtual_path_or_wide},
  windows::helpers::{WIN_FALSE, unhooked_fs},
};

patch_fn!(CreateDirectoryW, (PCWSTR, *const SECURITY_ATTRIBUTES) -> BOOL, detour_create_directory_w);

unsafe extern "system" fn detour_create_directory_w(
  path: PCWSTR,
  security_attributes: *const SECURITY_ATTRIBUTES,
) -> BOOL {
  trace_expr!(WIN_FALSE, unsafe {
    let mut virtual_path_res = get_virtual_path_or_wide(path)?;

    // check that folder doesn't exist at original path
    if let Ok(virtual_path) = &virtual_path_res
      && unhooked_fs::path_exists(virtual_path.original.as_path(), true)?
    {
      return Ok(WIN_FALSE);
    }

    Ok(original_create_directory_w(
      virtual_path_res.as_raw_or_original(),
      security_attributes,
    ))
  })
}

patch_fn!(RemoveDirectoryW, (PCWSTR) -> BOOL, detour_remove_directory_w);

unsafe extern "system" fn detour_remove_directory_w(path: PCWSTR) -> BOOL {
  trace_expr!(BOOL(0), unsafe {
    let mut virtual_path_res = get_virtual_path_or_wide(path)?;

    let res = original_remove_directory_w(virtual_path_res.as_raw_or_original());

    if res.as_bool() && virtual_path_res.is_ok() {
      Ok(original_remove_directory_w(path))
    } else {
      Ok(res)
    }
  })
}
