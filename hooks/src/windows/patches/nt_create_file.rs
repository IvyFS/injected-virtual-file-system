use std::ffi::c_void;

use proc_macros::patch_fn;
use shared_types::HookError;
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
  log::trace_inspect,
  virtual_paths::windows::get_virtual_path,
  windows::helpers::{
    handles::{HANDLE_MAP, ObjectAttributesExt},
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
  original_attrs: *const OBJECT_ATTRIBUTES,
  _3: *mut IO_STATUS_BLOCK,
  _4: *const i64,
  flags_and_attributes: FILE_FLAGS_AND_ATTRIBUTES,
  share_mode: FILE_SHARE_MODE,
  _7: NTCREATEFILE_CREATE_DISPOSITION,
  _8: NTCREATEFILE_CREATE_OPTIONS,
  _9: *const c_void,
  _10: u32,
) -> NTSTATUS {
  let path = unsafe { original_attrs.path() };
  let virtual_res = trace_inspect!(unsafe {
    let path = path.as_ref().map_err(Clone::clone)?;
    let virtual_path = get_virtual_path(path)?.ok_or(HookError::NoVirtualPath)?;
    let owned_attrs = original_attrs.reroute(virtual_path.path)?;
    let attrs = &raw const owned_attrs.attrs;

    let res = original_nt_create_file(
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
      Ok(Ok((owned_attrs, res)))
    } else {
      Ok(Err(res))
    }
  });

  if let Ok(Ok((owned_attrs, res))) = virtual_res {
    HANDLE_MAP.insert(
      handle,
      owned_attrs.unicode_path.string_buffer.to_os_string(),
      true,
    );
    res
  } else {
    let res = unsafe {
      original_nt_create_file(
        handle,
        _1,
        original_attrs,
        _3,
        _4,
        flags_and_attributes,
        share_mode,
        _7,
        _8,
        _9,
        _10,
      )
    };
    if res.is_ok()
      && let Ok(path) = path
    {
      HANDLE_MAP.insert(handle, path, false);
    }
    res
  }
}
