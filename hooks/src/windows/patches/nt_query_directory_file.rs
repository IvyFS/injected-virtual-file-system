use std::{ffi::c_void, sync::LazyLock};

use dashmap::DashMap;

use proc_macros::patch_fn;
use shared_types::HookError;
use win_api::{
  Wdk::{Storage::FileSystem::FILE_INFORMATION_CLASS, System::SystemServices::SL_RESTART_SCAN},
  Win32::{
    Foundation::{HANDLE, NTSTATUS, STATUS_NO_SUCH_FILE, UNICODE_STRING},
    System::IO::{IO_STATUS_BLOCK, PIO_APC_ROUTINE},
  },
};

use crate::{
  extension_traits::DashExt,
  log::{logfmt_dbg, trace_expr},
  windows::{
    os_types::handles::{
      HANDLE_MAP, Handle, get_virtual_path, into_handle, std_open_dir_handle_unhooked,
    },
    patches::{FIND_FIRST_FILE_ACTIVE, nt_close_unhooked},
  },
};

pub static QUERY_MAP: LazyLock<QueryMap> = LazyLock::new(Default::default);

#[derive(Debug, Default)]
pub struct QueryMap(DashMap<Handle, Handle>);

impl QueryMap {
  fn get_or_insert_query<I: Into<Handle>>(
    &self,
    real_ptr: into_handle!(),
    make_virtual_ptr: impl FnOnce() -> Result<I, HookError>,
  ) -> Result<dashmap::mapref::one::RefMut<'_, Handle, Handle>, HookError> {
    self
      .0
      .get_or_try_insert_with(real_ptr.into(), || make_virtual_ptr().map(Into::into))
  }

  pub fn remove_query(&self, real_ptr: into_handle!()) -> Option<(Handle, Handle)> {
    self.0.remove(&real_ptr.into())
  }
}

patch_fn!(
  NtQueryDirectoryFile,
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
  FIND_FIRST_FILE_ACTIVE.with(|flag| {
    if flag.load(std::sync::atomic::Ordering::Relaxed)
      && let Some(info) = HANDLE_MAP.get_by_handle(handle)
    {
      logfmt_dbg!("path: {:?}", info.path)
    }
  });
  let res = trace_expr!(STATUS_NO_SUCH_FILE, unsafe {
    let original_fn = nt_query_directory_file::original();
    let res = if let Some(virtual_path) = dbg!(HANDLE_MAP.get_by_handle(handle))
      .map(|p| dbg!(get_virtual_path(p.path.as_path())))
      .transpose()?
      .flatten()
    {
      if restart_scan && let Some((_, query_handle)) = QUERY_MAP.remove_query(handle) {
        nt_close_unhooked(*query_handle)
      }

      let virtual_handle = QUERY_MAP
        .get_or_insert_query(handle, || std_open_dir_handle_unhooked(virtual_path.path))?;

      original_fn(
        **virtual_handle,
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
    } else {
      original_fn(
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
    };

    Ok(res)
  });

  res
}

patch_fn!(
  NtQueryDirectoryFileEx,
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
  let res = trace_expr!(STATUS_NO_SUCH_FILE, unsafe {
    let original_fn = nt_query_directory_file_ex::original();
    let res = if let Some(virtual_path) = HANDLE_MAP
      .get_by_handle(handle)
      .inspect(|info| {
        if info.rerouted {
          logfmt_dbg!("{:?}", info.path)
        }
      })
      .map(|p| get_virtual_path(p.path.as_path()))
      .transpose()?
      .flatten()
    {
      if query_flags & SL_RESTART_SCAN > 0
        && let Some((_, query_handle)) = QUERY_MAP.remove_query(handle)
      {
        nt_close_unhooked(*query_handle)
      }

      let virtual_handle = QUERY_MAP
        .get_or_insert_query(handle, || std_open_dir_handle_unhooked(virtual_path.path))?;

      original_fn(
        **virtual_handle,
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
    } else {
      original_fn(
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
    };

    if let Some(info) = HANDLE_MAP.get_by_handle(handle) {
      logfmt_dbg!("filename filter: {:?}", filename.as_ref().map(|filter| filter.Buffer.to_string()));
      logfmt_dbg!("{:?} res: {:x}", info.path, res.0);
    }
    Ok(res)
  });

  res
}
