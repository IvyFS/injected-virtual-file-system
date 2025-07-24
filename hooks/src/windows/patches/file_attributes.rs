use std::{
  ffi::{OsString, c_void},
  os::windows::ffi::OsStringExt,
  path::{Path, PathBuf},
};

use proc_macros::patch_fn;
use shared_types::HookError;
use win_api::Win32::Storage::FileSystem::{
  FILE_FLAGS_AND_ATTRIBUTES, GET_FILEEX_INFO_LEVELS, INVALID_FILE_ATTRIBUTES,
};
use win_types::{BOOL, PCSTR, PCWSTR};

use crate::{log::trace_expr, windows::os_types::paths::get_virtual_path};

patch_fn! {
  GetFileAttributesA,
  (PCSTR) -> u32,
  detour_get_file_attributes_a
}

unsafe extern "system" fn detour_get_file_attributes_a(filename: PCSTR) -> u32 {
  trace_expr!(INVALID_FILE_ATTRIBUTES, unsafe {
    let (path, _virtual_guard) = ansi_virtual_path(filename)?;

    Ok(original_get_file_attributes_a(path))
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
    let (path, _virtual_guard) = ansi_virtual_path(filename)?;

    Ok(original_get_file_attributes_ex_a(
      path,
      info_level_id,
      file_information,
    ))
  })
}

#[track_caller]
fn ansi_virtual_path(filename: PCSTR) -> Result<(PCSTR, Option<Box<[u8]>>), HookError> {
  let given_path = { Path::new(unsafe { str::from_utf8_unchecked(filename.as_bytes()) }) };
  let canon_path = given_path
    .is_relative()
    .then(|| {
      std::env::current_dir().and_then(|current_dir| {
        let joined = current_dir.join(&given_path);
        joined.normalize_lexically().map_err(std::io::Error::other)
      })
    })
    .transpose()?;

  let virtual_path = get_virtual_path(canon_path.as_deref().unwrap_or(given_path))?.map(|virt| {
    virt
      .path
      .as_os_str()
      .as_encoded_bytes()
      .to_vec()
      .into_boxed_slice()
  });

  Ok((
    virtual_path
      .as_ref()
      .map(|virt| PCSTR::from_raw(virt.as_ptr()))
      .unwrap_or(filename),
    virtual_path,
  ))
}

patch_fn! {
  GetFileAttributesW,
  (PCWSTR) -> u32,
  detour_get_file_attributes_w
}

unsafe extern "system" fn detour_get_file_attributes_w(filename: PCWSTR) -> u32 {
  trace_expr!(INVALID_FILE_ATTRIBUTES, unsafe {
    let (path, _virtual_guard) = wide_virtual_path(filename)?;

    Ok(original_get_file_attributes_w(path))
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
    let (path, _virtual_guard) = wide_virtual_path(filename)?;

    Ok(original_get_file_attributes_ex_w(
      path,
      info_level_id,
      file_information,
    ))
  })
}

#[track_caller]
fn wide_virtual_path(
  filename: PCWSTR,
) -> Result<(PCWSTR, Option<widestring::U16CString>), HookError> {
  let given_path = PathBuf::from(unsafe { OsString::from_wide(filename.as_wide()) });
  let canon_path = given_path
    .is_relative()
    .then(|| {
      std::env::current_dir().and_then(|current_dir| {
        current_dir
          .join(&given_path)
          .normalize_lexically()
          .map_err(std::io::Error::other)
      })
    })
    .transpose()?;
  let virtual_path = get_virtual_path(canon_path.unwrap_or(given_path))?
    .map(|virt| widestring::U16CString::from_os_str_truncate(virt.path));

  Ok((
    virtual_path
      .as_ref()
      .map(|virt| PCWSTR::from_raw(virt.as_ptr()))
      .unwrap_or(filename),
    virtual_path,
  ))
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
    let (path, _virtual_guard) = wide_virtual_path(filename)?;

    let res = original_set_file_attributes_w(path, file_attributes);

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
