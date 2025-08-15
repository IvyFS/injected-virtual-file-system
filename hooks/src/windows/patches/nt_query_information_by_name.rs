use std::{ffi::c_void, path::PathBuf};

use proc_macros::patch_fn;
use win_api::{
  Wdk::{Foundation::OBJECT_ATTRIBUTES, Storage::FileSystem::FILE_INFORMATION_CLASS},
  Win32::{
    Foundation::{NTSTATUS, STATUS_NO_SUCH_FILE},
    System::IO::IO_STATUS_BLOCK,
  },
};

use crate::{
  log::trace_expr,
  virtual_paths::windows::get_virtual_path,
  windows::helpers::{handles::ObjectAttributesExt, object_attributes::RawObjectAttrsExt},
};

patch_fn! {
  NtQueryInformationByName,
  (
    *const OBJECT_ATTRIBUTES,
    *mut IO_STATUS_BLOCK,
    *mut c_void,
    u32,
    FILE_INFORMATION_CLASS,
  ) -> NTSTATUS,
  detour_nt_query_information_by_name
}

unsafe extern "system" fn detour_nt_query_information_by_name(
  attrs: *const OBJECT_ATTRIBUTES,
  io_status_block: *mut IO_STATUS_BLOCK,
  file_information_buffer: *mut c_void,
  buf_size: u32,
  file_information_class: FILE_INFORMATION_CLASS,
) -> NTSTATUS {
  trace_expr!(STATUS_NO_SUCH_FILE, unsafe {
    let path: PathBuf = attrs.path()?;
    let (attrs_ptr, _reroute_guard) = if let Some(virtual_path) = get_virtual_path(&path)? {
      let attrs = attrs.reroute(virtual_path.path)?;
      (&raw const attrs.attrs, Some(attrs))
    } else {
      (attrs, None)
    };

    let res = original_nt_query_information_by_name(
      attrs_ptr,
      io_status_block,
      file_information_buffer,
      buf_size,
      file_information_class,
    );

    Ok(res)
  })
}
