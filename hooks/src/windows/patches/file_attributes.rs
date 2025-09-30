use std::ffi::c_void;

use proc_macros::patch_fn;
use win_api::Win32::Storage::FileSystem::{
  FILE_FLAGS_AND_ATTRIBUTES, GET_FILEEX_INFO_LEVELS, INVALID_FILE_ATTRIBUTES,
};
use win_types::{BOOL, PCSTR, PCWSTR};

use crate::{
  log::trace_expr,
  virtual_paths::windows::{VirtualPathOption, get_virtual_path_or_ansi, get_virtual_path_or_wide},
};

patch_fn! {
  GetFileAttributesA,
  (PCSTR) -> u32,
  detour_get_file_attributes_a
}

unsafe extern "system" fn detour_get_file_attributes_a(filename: PCSTR) -> u32 {
  trace_expr!(INVALID_FILE_ATTRIBUTES, unsafe {
    let mut virtual_path_res = get_virtual_path_or_ansi(filename)?;

    Ok(original_get_file_attributes_a(
      virtual_path_res.as_raw_or_original(),
    ))
  })
}

patch_fn! {
  GetFileAttributesExA,
  (PCSTR, GET_FILEEX_INFO_LEVELS, *mut c_void) -> u32,
  detour_get_file_attributes_ex_a
}

unsafe extern "system" fn detour_get_file_attributes_ex_a(
  filename: PCSTR,
  info_level_id: GET_FILEEX_INFO_LEVELS,
  file_information: *mut c_void,
) -> u32 {
  trace_expr!(INVALID_FILE_ATTRIBUTES, unsafe {
    let mut virtual_path_res = get_virtual_path_or_ansi(filename)?;

    Ok(original_get_file_attributes_ex_a(
      virtual_path_res.as_raw_or_original(),
      info_level_id,
      file_information,
    ))
  })
}

patch_fn! {
  GetFileAttributesW,
  (PCWSTR) -> u32,
  detour_get_file_attributes_w
}

unsafe extern "system" fn detour_get_file_attributes_w(filename: PCWSTR) -> u32 {
  trace_expr!(INVALID_FILE_ATTRIBUTES, unsafe {
    let mut virtual_path_res = get_virtual_path_or_wide(filename)?;

    Ok(original_get_file_attributes_w(
      virtual_path_res.as_raw_or_original(),
    ))
  })
}

patch_fn! {
  GetFileAttributesExW,
  (PCWSTR, GET_FILEEX_INFO_LEVELS, *mut c_void) -> u32,
  detour_get_file_attributes_ex_w
}

unsafe extern "system" fn detour_get_file_attributes_ex_w(
  filename: PCWSTR,
  info_level_id: GET_FILEEX_INFO_LEVELS,
  file_information: *mut c_void,
) -> u32 {
  trace_expr!(INVALID_FILE_ATTRIBUTES, unsafe {
    let mut virtual_path_res = get_virtual_path_or_wide(filename)?;

    Ok(original_get_file_attributes_ex_w(
      virtual_path_res.as_raw_or_original(),
      info_level_id,
      file_information,
    ))
  })
}

patch_fn! {
  SetFileAttributesW,
  (PCWSTR, FILE_FLAGS_AND_ATTRIBUTES) -> BOOL,
  detour_set_file_attributes_w
}

unsafe extern "system" fn detour_set_file_attributes_w(
  filename: PCWSTR,
  file_attributes: FILE_FLAGS_AND_ATTRIBUTES,
) -> BOOL {
  trace_expr!(BOOL(0), unsafe {
    let mut virtual_path_res = get_virtual_path_or_wide(filename)?;

    let res =
      original_set_file_attributes_w(virtual_path_res.as_raw_or_original(), file_attributes);

    Ok(res)
  })
}

#[cfg(test)]
mod test {
  #[cfg(windows)]
  #[test]
  fn windows_encode_osstr_as_utf8() {
    use std::ffi::OsStr;

    let os_str = OsStr::new("C:\\foo\\bar.baz");

    assert_eq!(b"C:\\foo\\bar.baz", os_str.as_encoded_bytes())
  }
}
