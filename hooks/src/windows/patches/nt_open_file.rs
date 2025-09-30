use proc_macros::patch_fn;
use shared_types::HookError;
use win_api::{
  Wdk::Foundation::OBJECT_ATTRIBUTES,
  Win32::{
    Foundation::{HANDLE, NTSTATUS},
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
  "ntdll.dll",
  NtOpenFile,
  (
    *mut HANDLE,
    u32,
    *const OBJECT_ATTRIBUTES,
    *mut IO_STATUS_BLOCK,
    u32,
    u32
  ) -> NTSTATUS,
  detour_nt_open_file
);

pub unsafe extern "system" fn detour_nt_open_file(
  filehandle: *mut HANDLE,
  desiredaccess: u32,
  original_attrs: *const OBJECT_ATTRIBUTES,
  iostatusblock: *mut IO_STATUS_BLOCK,
  shareaccess: u32,
  openoptions: u32,
) -> NTSTATUS {
  let path = unsafe { original_attrs.path() };
  let virtual_res = trace_inspect!(unsafe {
    let path = path.as_ref().map_err(Clone::clone)?;
    let virtual_path = get_virtual_path(path)?.ok_or(HookError::NoVirtualPath)?;
    let owned_attrs = original_attrs.reroute(virtual_path.path)?;
    let attrs = &raw const owned_attrs.attrs;

    let res = original_nt_open_file(
      filehandle,
      desiredaccess,
      attrs,
      iostatusblock,
      shareaccess,
      openoptions,
    );

    if res.is_ok() {
      Ok(Ok((owned_attrs, res)))
    } else {
      Ok(Err(res))
    }
  });

  if let Ok(Ok((owned_attrs, res))) = virtual_res {
    HANDLE_MAP.insert(
      filehandle,
      owned_attrs.unicode_path.string_buffer.to_os_string(),
      true,
    );
    res
  } else {
    let res = unsafe {
      original_nt_open_file(
        filehandle,
        desiredaccess,
        original_attrs,
        iostatusblock,
        shareaccess,
        openoptions,
      )
    };
    if res.is_ok()
      && let Ok(path) = path
    {
      HANDLE_MAP.insert(filehandle, path, false);
    }
    res
  }
}
