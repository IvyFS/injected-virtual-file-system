use std::ffi::c_void;

use macros::{crabtime, generate_patch};
use win_api::Win32::{
  Foundation::{HANDLE, INVALID_HANDLE_VALUE},
  Storage::FileSystem::{FINDEX_INFO_LEVELS, FINDEX_SEARCH_OPS},
};
use win_types::PCWSTR;

use crate::log::{logfmt_dbg, trace_expr};

generate_patch!(
  "FindFirstFileExW",
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
  filename: PCWSTR,
  info_level_id: FINDEX_INFO_LEVELS,
  find_file_data: *mut c_void,
  search_ops: FINDEX_SEARCH_OPS,
  search_filters: *const c_void,
  additional_flags: u32,
) -> HANDLE {
  let expr = trace_expr!(INVALID_HANDLE_VALUE, unsafe {
    logfmt_dbg!("{:?}", filename.to_string());

    let original_fn = original();

    let handle = original_fn(
      filename,
      info_level_id,
      find_file_data,
      search_ops,
      search_filters,
      additional_flags,
    );

    Ok(handle)
  });

  expr
}
