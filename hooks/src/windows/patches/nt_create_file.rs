use std::{ffi::c_void, path::Path};

use macros::{crabtime, generate_patch};
pub use win_api::{
  Wdk::{
    Foundation::OBJECT_ATTRIBUTES,
    Storage::FileSystem::{NTCREATEFILE_CREATE_DISPOSITION, NTCREATEFILE_CREATE_OPTIONS},
  },
  Win32::{
    Foundation::{HANDLE, NTSTATUS},
    Storage::FileSystem::{FILE_ACCESS_RIGHTS, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE},
    System::IO::IO_STATUS_BLOCK,
  },
};

use crate::log::*;
use crate::{
  virtual_paths::MOUNT_POINT,
  windows::handles::{HandleMap, ObjectAttributesExt},
};
pub use nt_create_file::*;

generate_patch!(
  "NtCreateFile",
  (
    *mut HANDLE,
    FILE_ACCESS_RIGHTS,
    *const OBJECT_ATTRIBUTES,
    *mut IO_STATUS_BLOCK,
    *const i64,
    FILE_FLAGS_AND_ATTRIBUTES,
    FILE_SHARE_MODE,
    NTCREATEFILE_CREATE_DISPOSITION,
    NTCREATEFILE_CREATE_OPTIONS,
    *const c_void,
    u32
  ) -> NTSTATUS,
  detour_nt_create_file
);

unsafe extern "system" fn detour_nt_create_file(
  handle: *mut HANDLE,
  _1: FILE_ACCESS_RIGHTS,
  attrs: *const OBJECT_ATTRIBUTES,
  _3: *mut IO_STATUS_BLOCK,
  _4: *const i64,
  _5: FILE_FLAGS_AND_ATTRIBUTES,
  _6: FILE_SHARE_MODE,
  _7: NTCREATEFILE_CREATE_DISPOSITION,
  _8: NTCREATEFILE_CREATE_OPTIONS,
  _9: *const c_void,
  _10: u32,
) -> NTSTATUS {
  let original = unsafe { get_original() };

  let res = unsafe { original(handle, _1, attrs, _3, _4, _5, _6, _7, _8, _9, _10) };

  trace!(unsafe {
    if res.is_ok() {
      HandleMap::update_handles(*handle, attrs)?;
    }
    let path_str = (&*attrs).path()?;
    if Path::new(&path_str).starts_with(MOUNT_POINT.lock()?.as_path()) {
      log_info(format!("(Sub-)path of mount point: {path_str}"));
    }
  });

  res
}
