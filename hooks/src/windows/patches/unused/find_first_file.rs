use std::ffi::c_void;

use proc_macros::patch_fn;
use win_api::Win32::{
  Foundation::{HANDLE, INVALID_HANDLE_VALUE},
  Storage::FileSystem::{FINDEX_INFO_LEVELS, FINDEX_SEARCH_OPS},
};
use win_types::PCWSTR;

use crate::{
  log::{logfmt_dbg, trace_expr},
  windows::os_types::handles::HANDLE_MAP,
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
    let handle = original()(
      query_filename,
      info_level_id,
      find_file_data,
      search_ops,
      search_filters,
      additional_flags,
    );

    if !handle.is_invalid() {
      logfmt_dbg!("nt_find_first ptr {:p}", handle.0);
      if let Some(info) = HANDLE_MAP.get_by_handle(handle) {
        logfmt_dbg!("path: {:?}", info.path)
      }
    }

    Ok(handle)
  });

  expr
}
