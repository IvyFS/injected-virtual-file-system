use std::ffi::c_void;

use proc_macros::patch_fn;
use win_api::Win32::Storage::FileSystem::{COPYFILE_FLAGS, LPPROGRESS_ROUTINE, MOVE_FILE_FLAGS};
use win_types::{BOOL, PCSTR, PCWSTR};

use crate::{
  log::trace_expr,
  virtual_paths::windows::{VirtualPathOption, get_virtual_path_or_ansi, get_virtual_path_or_wide},
  windows::helpers::WIN_FALSE,
};

patch_fn!(
  DeleteFileW,
  (PCWSTR) -> BOOL,
  detour_delete_file_w
);

// TODO: delete "mounted" file at this path as well
unsafe extern "system" fn detour_delete_file_w(path: PCWSTR) -> BOOL {
  trace_expr!(WIN_FALSE, unsafe {
    Ok(original_delete_file_w(
      get_virtual_path_or_wide(path)?.as_raw_or_original(),
    ))
  })
}

patch_fn!(MoveFileA, (PCSTR, PCSTR) -> BOOL, detour_move_file_a);

unsafe extern "system" fn detour_move_file_a(source: PCSTR, dest: PCSTR) -> BOOL {
  trace_expr!(WIN_FALSE, unsafe {
    let mut source_res = get_virtual_path_or_ansi(source)?;
    let mut dest_res = get_virtual_path_or_ansi(dest)?;

    Ok(original_move_file_a(
      source_res.as_raw_or_original(),
      dest_res.as_raw_or_original(),
    ))
  })
}

patch_fn!(MoveFileExA, (PCSTR, PCSTR, MOVE_FILE_FLAGS) -> BOOL, detour_move_file_ex_a);

// TODO: moving a directory (which is done with MoveFileEx, etc) currently doesn't move any files in the mounted
// directory with the virtual directory

unsafe extern "system" fn detour_move_file_ex_a(
  source: PCSTR,
  dest: PCSTR,
  flags: MOVE_FILE_FLAGS,
) -> BOOL {
  trace_expr!(WIN_FALSE, unsafe {
    let mut source_res = get_virtual_path_or_ansi(source)?;
    let mut dest_res = get_virtual_path_or_ansi(dest)?;

    Ok(original_move_file_ex_a(
      source_res.as_raw_or_original(),
      dest_res.as_raw_or_original(),
      flags,
    ))
  })
}

patch_fn!(MoveFileW, (PCWSTR, PCWSTR) -> BOOL, detour_move_file_w);

unsafe extern "system" fn detour_move_file_w(source: PCWSTR, dest: PCWSTR) -> BOOL {
  trace_expr!(WIN_FALSE, unsafe {
    let mut source_res = get_virtual_path_or_wide(source)?;
    let mut dest_res = get_virtual_path_or_wide(dest)?;

    Ok(original_move_file_w(
      source_res.as_raw_or_original(),
      dest_res.as_raw_or_original(),
    ))
  })
}

patch_fn!(MoveFileExW, (PCWSTR, PCWSTR, MOVE_FILE_FLAGS) -> BOOL, detour_move_file_ex_w);

unsafe extern "system" fn detour_move_file_ex_w(
  source: PCWSTR,
  dest: PCWSTR,
  flags: MOVE_FILE_FLAGS,
) -> BOOL {
  trace_expr!(WIN_FALSE, unsafe {
    let mut source_res = get_virtual_path_or_wide(source)?;
    let mut dest_res = get_virtual_path_or_wide(dest)?;

    Ok(original_move_file_ex_w(
      source_res.as_raw_or_original(),
      dest_res.as_raw_or_original(),
      flags,
    ))
  })
}

patch_fn!(
  MoveFileWithProgressA,
  (
    PCSTR,
    PCSTR,
    LPPROGRESS_ROUTINE,
    *mut c_void,
    MOVE_FILE_FLAGS
  ) -> BOOL,
  detour_move_file_with_progress_a
);

unsafe extern "system" fn detour_move_file_with_progress_a(
  source: PCSTR,
  dest: PCSTR,
  callback: LPPROGRESS_ROUTINE,
  data: *mut c_void,
  flags: MOVE_FILE_FLAGS,
) -> BOOL {
  trace_expr!(WIN_FALSE, unsafe {
    let mut source_res = get_virtual_path_or_ansi(source)?;
    let mut dest_res = get_virtual_path_or_ansi(dest)?;

    Ok(original_move_file_with_progress_a(
      source_res.as_raw_or_original(),
      dest_res.as_raw_or_original(),
      callback,
      data,
      flags,
    ))
  })
}

patch_fn!(
  MoveFileWithProgressW,
  (
    PCWSTR,
    PCWSTR,
    LPPROGRESS_ROUTINE,
    *mut c_void,
    MOVE_FILE_FLAGS
  ) -> BOOL,
  detour_move_file_with_progress_w
);

unsafe extern "system" fn detour_move_file_with_progress_w(
  source: PCWSTR,
  dest: PCWSTR,
  callback: LPPROGRESS_ROUTINE,
  data: *mut c_void,
  flags: MOVE_FILE_FLAGS,
) -> BOOL {
  trace_expr!(WIN_FALSE, unsafe {
    let mut source_res = get_virtual_path_or_wide(source)?;
    let mut dest_res = get_virtual_path_or_wide(dest)?;

    Ok(original_move_file_with_progress_w(
      source_res.as_raw_or_original(),
      dest_res.as_raw_or_original(),
      callback,
      data,
      flags,
    ))
  })
}

patch_fn!(
  CopyFileExW,
  (
    PCWSTR,
    PCWSTR,
    LPPROGRESS_ROUTINE,
    *const c_void,
    *mut BOOL,
    COPYFILE_FLAGS,
  ) -> BOOL,
  detour_copy_file_ex_w
);

unsafe extern "system" fn detour_copy_file_ex_w(
  source: PCWSTR,
  dest: PCWSTR,
  callback: LPPROGRESS_ROUTINE,
  cb_data: *const c_void,
  pbcancel: *mut BOOL,
  flags: COPYFILE_FLAGS,
) -> BOOL {
  trace_expr!(WIN_FALSE, unsafe {
    let mut source_res = get_virtual_path_or_wide(source)?;
    let mut dest_res = get_virtual_path_or_wide(dest)?;

    Ok(original_copy_file_ex_w(
      source_res.as_raw_or_original(),
      dest_res.as_raw_or_original(),
      callback,
      cb_data,
      pbcancel,
      flags,
    ))
  })
}
