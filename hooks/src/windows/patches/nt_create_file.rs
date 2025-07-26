use std::{ffi::c_void, path::PathBuf};

use proc_macros::patch_fn;
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
  log::{logfmt_dbg, trace_expr},
  virtual_paths::windows::get_virtual_path,
  windows::os_types::{
    handles::{DO_NOT_HOOK, HANDLE_MAP, ObjectAttributesExt},
    object_attributes::RawObjectAttrsExt,
  },
};

patch_fn!(
  NtCreateFile,
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

pub(crate) unsafe extern "system" fn detour_nt_create_file(
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
  trace_expr!(STATUS_NO_SUCH_FILE, unsafe {
    let path: PathBuf = attrs.path()?;
    let (attrs_ptr, reroute_guard) = if flags_and_attributes.contains(DO_NOT_HOOK) {
      flags_and_attributes.0 ^= DO_NOT_HOOK.0;
      logfmt_dbg!("Got DO_NOT_HOOK for path {:?}", attrs);
      (attrs, None)
    } else if let Some(virtual_path) = get_virtual_path(&path)? {
      let attrs = attrs.reroute(virtual_path.path)?;
      (&raw const attrs.attrs, Some(attrs))
    } else {
      (attrs, None)
    };

    let status = original_nt_create_file(
      handle,
      _1,
      attrs_ptr,
      _3,
      _4,
      flags_and_attributes,
      share_mode,
      _7,
      _8,
      _9,
      _10,
    );

    if status.is_ok() {
      if let Some(reroute) = reroute_guard {
        logfmt_dbg!(
          "rerouting from {:?} to {:?}",
          path,
          reroute.unicode_path.string_buffer
        );
        HANDLE_MAP.insert(
          handle,
          reroute.unicode_path.string_buffer.to_os_string(),
          true,
        )
      } else {
        HANDLE_MAP.insert(handle, path, false)
      };
    }

    Ok(status)
  })
}
