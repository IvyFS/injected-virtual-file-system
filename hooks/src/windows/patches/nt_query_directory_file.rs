use std::ffi::c_void;

use macros::{crabtime, generate_patch};

use win_api::{
  Wdk::Storage::FileSystem::FILE_INFORMATION_CLASS,
  Win32::{
    Foundation::{HANDLE, NTSTATUS, UNICODE_STRING},
    System::IO::{IO_STATUS_BLOCK, PIO_APC_ROUTINE},
  },
};

pub use nt_query_directory_file::*;
pub use nt_query_directory_file_ex::*;

use crate::{
  log::{log_info, trace},
  windows::handles::path_from_handle,
};

generate_patch!(
  "NtQueryDirectoryFile",
  (
    HANDLE,
    HANDLE,
    PIO_APC_ROUTINE,
    *const c_void,
    *mut IO_STATUS_BLOCK,
    *mut c_void,
    u32,
    FILE_INFORMATION_CLASS,
    bool,
    *const UNICODE_STRING,
    bool
  ) -> NTSTATUS,
  detour_nt_query_directory_file
);

unsafe extern "system" fn detour_nt_query_directory_file(
  handle: HANDLE,
  event: HANDLE,
  apc_routine: PIO_APC_ROUTINE,
  apc_context: *const c_void,
  io_status_block: *mut IO_STATUS_BLOCK,
  file_information: *mut c_void,
  length: u32,
  file_information_class: FILE_INFORMATION_CLASS,
  return_single_entry: bool,
  filename: *const UNICODE_STRING,
  restart_scan: bool,
) -> NTSTATUS {
  unsafe {
    let original = ORIGINAL_NT_QUERY_DIRECTORY_FILE
      .get()
      .unwrap()
      .lock()
      .unwrap();

    trace!({
      log_info(format!(
        "NtQueryDirectoryFile: {}",
        path_from_handle(&handle)?
      ));
    });

    original(
      handle,
      event,
      apc_routine,
      apc_context,
      io_status_block,
      file_information,
      length,
      file_information_class,
      return_single_entry,
      filename,
      restart_scan,
    )
  }
}

generate_patch!(
  "NtQueryDirectoryFileEx",
  (
    HANDLE,
    HANDLE,
    PIO_APC_ROUTINE,
    *const c_void,
    *mut IO_STATUS_BLOCK,
    *mut c_void,
    u32,
    FILE_INFORMATION_CLASS,
    u32,
    *const UNICODE_STRING
  ) -> NTSTATUS,
  detour_nt_query_directory_file_ex
);

unsafe extern "system" fn detour_nt_query_directory_file_ex(
  handle: HANDLE,
  event: HANDLE,
  apc_routine: PIO_APC_ROUTINE,
  apc_context: *const c_void,
  io_status_block: *mut IO_STATUS_BLOCK,
  file_information: *mut c_void,
  length: u32,
  file_information_class: FILE_INFORMATION_CLASS,
  query_flags: u32,
  filename: *const UNICODE_STRING,
) -> NTSTATUS {
  unsafe {
    let original = ORIGINAL_NT_QUERY_DIRECTORY_FILE_EX
      .get()
      .unwrap()
      .lock()
      .unwrap();

    trace!({
      log_info(format!(
        "NtQueryDirectoryFileEx: {}",
        path_from_handle(&handle)?
      ));
    });

    original(
      handle,
      event,
      apc_routine,
      apc_context,
      io_status_block,
      file_information,
      length,
      file_information_class,
      query_flags,
      filename,
    )
  }
}
