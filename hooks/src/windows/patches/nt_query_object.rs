use macros::{crabtime, generate_patch};
use shared_types::Message;
use win_api::{
  Wdk::Foundation::OBJECT_INFORMATION_CLASS,
  Win32::Foundation::{HANDLE, NTSTATUS},
};

use crate::{
  log::{log_info, log_lossy},
  windows::os_types::handles::{HANDLE_MAP, HandleMap, path_from_handle},
};

generate_patch!(
  "NtQueryObject",
  (
    HANDLE,
    OBJECT_INFORMATION_CLASS,
    *mut std::ffi::c_void,
    u32,
    *mut u32
  ) -> NTSTATUS,
  detour_nt_query_object
);

unsafe extern "system" fn detour_nt_query_object(
  handle: HANDLE,
  object_information_class: OBJECT_INFORMATION_CLASS,
  object_information: *mut std::ffi::c_void,
  information_length: u32,
  return_length: *mut u32,
) -> NTSTATUS {
  unsafe {
    let original = nt_query_object::original();

    let res = original(
      handle,
      object_information_class,
      object_information,
      information_length,
      return_length,
    );

    if let Some(info) = HANDLE_MAP.get_by_handle(handle) {
      // log_lossy(Message::DebugInfo(format!("Query object: {}", info.path)));
    }

    res
  }
}
