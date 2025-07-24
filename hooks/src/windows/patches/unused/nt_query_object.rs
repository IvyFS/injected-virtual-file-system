use proc_macros::patch_fn;
use win_api::{
  Wdk::Foundation::OBJECT_INFORMATION_CLASS,
  Win32::Foundation::{HANDLE, NTSTATUS, STATUS_INVALID_HANDLE},
};

use crate::{log::trace_expr, windows::os_types::handles::HANDLE_MAP};

patch_fn!(
  NtQueryObject,
  (
    HANDLE,
    OBJECT_INFORMATION_CLASS,
    *mut std::ffi::c_void,
    u32,
    *mut u32
  ) -> NTSTATUS,
  // detour_nt_query_object
);

unsafe extern "system" fn detour_nt_query_object(
  handle: HANDLE,
  object_information_class: OBJECT_INFORMATION_CLASS,
  object_information: *mut std::ffi::c_void,
  information_length: u32,
  return_length: *mut u32,
) -> NTSTATUS {
  trace_expr!(STATUS_INVALID_HANDLE, unsafe {
    let path = HANDLE_MAP.get_by_handle(handle).andt;

    let res = original_nt_query_object(
      handle,
      object_information_class,
      object_information,
      information_length,
      return_length,
    );

    Ok(res)
  })
}
