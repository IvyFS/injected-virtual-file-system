use std::ffi::c_void;

use macros::{crabtime, generate_patch};
use shared_types::{HookError, Message};
use win_api::Win32::Foundation::STATUS_NO_SUCH_FILE;
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

use crate::{
  log::*,
  windows::os_types::handles::{DO_NOT_HOOK, HandleMap, ObjectAttributesExt},
};

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
  mut flags_and_attributes: FILE_FLAGS_AND_ATTRIBUTES,
  share_mode: FILE_SHARE_MODE,
  _7: NTCREATEFILE_CREATE_DISPOSITION,
  _8: NTCREATEFILE_CREATE_OPTIONS,
  _9: *const c_void,
  _10: u32,
) -> NTSTATUS {
  let res = trace_expr!(STATUS_NO_SUCH_FILE, unsafe {
    if flags_and_attributes.contains(DO_NOT_HOOK) {
      flags_and_attributes.0 ^= DO_NOT_HOOK.0;
      logfmt_dbg!("Got DO_NOT_HOOK for path {:?}", attrs.path());
    }

    let original_fn = original();
    let res = original_fn(
      handle,
      _1,
      attrs,
      _3,
      _4,
      flags_and_attributes,
      share_mode,
      _7,
      _8,
      _9,
      _10,
    );

    if res.is_ok() {
      log_lossy(Message::DebugInfo(format!("nt_create {:?}", attrs.path())));
      HandleMap::insert_by_object_attributes(*handle, attrs)?;
    }

    Ok(res)
  });

  res
}
