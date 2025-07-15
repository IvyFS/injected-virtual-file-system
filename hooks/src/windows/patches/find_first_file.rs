use std::{
  ffi::{OsString, c_void},
  os::windows::ffi::OsStringExt,
  path::Path,
};

use proc_macros::patch_fn;
use shared_types::HookError;
use win_api::Win32::{
  Foundation::{HANDLE, INVALID_HANDLE_VALUE},
  Storage::FileSystem::{FINDEX_INFO_LEVELS, FINDEX_SEARCH_OPS},
};
use win_types::PCWSTR;

use crate::{
  log::{logfmt_dbg, trace_expr},
  windows::os_types::{
    handles::{HANDLE_MAP, get_virtual_path},
    paths::fragment_is_relative,
  },
};

thread_local! {
  pub static FIND_FIRST_FILE_ACTIVE: std::sync::atomic::AtomicBool = Default::default();
}

patch_fn!(
  FindFirstFileExW,
  (
    PCWSTR,
    FINDEX_INFO_LEVELS,
    *mut c_void,
    FINDEX_SEARCH_OPS,
    *const c_void,
    u32
  ) -> HANDLE,
  detour_find_first_file_ex_w
);

unsafe extern "system" fn detour_find_first_file_ex_w(
  query_filename: PCWSTR,
  info_level_id: FINDEX_INFO_LEVELS,
  find_file_data: *mut c_void,
  search_ops: FINDEX_SEARCH_OPS,
  search_filters: *const c_void,
  additional_flags: u32,
) -> HANDLE {
  let expr = trace_expr!(INVALID_HANDLE_VALUE, unsafe {
    let os_str_filename = OsString::from_wide(query_filename.as_wide());
    let filename = Path::new(&os_str_filename);

    let path = if fragment_is_relative(&filename) {
      let path = std::env::current_dir()?
        .join(&filename)
        .normalize_lexically()
        .map_err(|err| HookError::Other(err.to_string()))?;
      logfmt_dbg!("relative normalised to: {:?}", path);
      path
    } else {
      filename.to_owned()
    };
    let reroute = get_virtual_path(&path)?;

    let (final_path, _filename_guard) = if let Some(rerouted) = reroute.as_ref() {
      let owned_path = widestring::U16CString::from_os_str_unchecked(&rerouted.path);
      (PCWSTR(owned_path.as_ptr()), Some(owned_path))
    } else {
      (query_filename, None)
    };

    let handle = original()(
      final_path,
      info_level_id,
      find_file_data,
      search_ops,
      search_filters,
      additional_flags,
    );

    if !handle.is_invalid()
      && let Some(reroute) = reroute
    {
      logfmt_dbg!("{:?}", reroute);
      HANDLE_MAP.overwrite(handle, reroute.path, true);
    }

    Ok(handle)
  });

  expr
}
