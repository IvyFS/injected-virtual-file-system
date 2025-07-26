use std::{ffi::c_void, sync::LazyLock};

use dashmap::DashMap;

use proc_macros::patch_fn;
use shared_types::HookError;
use win_api::{
  Wdk::{
    Storage::FileSystem::{
      FILE_BOTH_DIR_INFORMATION, FILE_DIRECTORY_INFORMATION, FILE_FULL_DIR_INFORMATION,
      FILE_INFORMATION_CLASS, FILE_NAMES_INFORMATION, FileBothDirectoryInformation,
      FileDirectoryInformation, FileFullDirectoryInformation, FileNamesInformation,
    },
    System::SystemServices::SL_RESTART_SCAN,
  },
  Win32::{
    Foundation::{HANDLE, NTSTATUS, STATUS_NO_SUCH_FILE, UNICODE_STRING},
    System::IO::{IO_STATUS_BLOCK, PIO_APC_ROUTINE},
  },
};

use crate::{
  extension_traits::DashExt,
  log::{logfmt_dbg, trace_expr},
  virtual_paths::windows::get_virtual_path,
  windows::{
    os_types::handles::{HANDLE_MAP, Handle, into_handle, std_open_dir_handle_unhooked},
    patches::nt_close_detour,
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
  original_handle: HANDLE,
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
  let res = trace_expr!(STATUS_NO_SUCH_FILE, unsafe {
    if restart_scan && let Some((_, query_handle)) = QUERY_MAP.remove_query(original_handle) {
      if query_handle.0 != original_handle.0 {
        nt_close_detour(*query_handle)
      }
    }

    let handle = if let Some(reroute_handle) = QUERY_MAP.0.get(&original_handle.into()) {
      **reroute_handle
    } else if let Some(virtual_path) = HANDLE_MAP
      .get_by_handle(original_handle)
      .map(|p| get_virtual_path(p.path.as_path()))
      .transpose()?
      .flatten()
    {
      **QUERY_MAP.get_or_insert_query(original_handle, || {
        std_open_dir_handle_unhooked(virtual_path.path)
      })?
    } else {
      original_handle
    };

    let res = original_nt_query_directory_file(
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
    );

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
  original_handle: HANDLE,
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
    if query_flags & SL_RESTART_SCAN > 0
      && let Some((_, query_handle)) = QUERY_MAP.remove_query(original_handle)
    {
      if query_handle.0 != original_handle.0 {
        nt_close_detour(*query_handle)
      }
    }

    let handle = if let Some(reroute_handle) = QUERY_MAP.0.get(&original_handle.into()) {
      **reroute_handle
    } else if let Some(virtual_path) = HANDLE_MAP
      .get_by_handle(original_handle)
      .map(|p| get_virtual_path(p.path.as_path()))
      .transpose()?
      .flatten()
    {
      **QUERY_MAP.get_or_insert_query(original_handle, || {
        std_open_dir_handle_unhooked(virtual_path.path)
      })?
    } else {
      original_handle
    };

    let res = original_nt_query_directory_file_ex(
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
    );

    if let Some(info) = HANDLE_MAP.get_by_handle(handle) {
      logfmt_dbg!(
        "{:?}",
        get_file_information_path(file_information_class, file_information, length)
      );
      logfmt_dbg!("{:?} res: {:x}", info.path, res.0);
    }
    Ok(res)
  });

  res
}

fn get_file_information_path(
  class: FILE_INFORMATION_CLASS,
  buffer: *mut c_void,
  _: u32,
) -> Vec<String> {
  unsafe fn buffer_ptr_offset_to_typed_ref<'a, T>(buffer: *mut c_void, offset: usize) -> &'a mut T {
    unsafe { &mut *(buffer.add(offset) as *mut T) }
  }

  unsafe {
    let mut filenames = Vec::new();
    let mut total_offset = Some(0);
    while let Some(offset) = total_offset.take() {
      let file_information = match class {
        val if val == FileDirectoryInformation => {
          buffer_ptr_offset_to_typed_ref::<FILE_DIRECTORY_INFORMATION>(buffer, offset)
            as &dyn FileInformationLike
        }
        val if val == FileFullDirectoryInformation => {
          buffer_ptr_offset_to_typed_ref::<FILE_FULL_DIR_INFORMATION>(buffer, offset) as _
        }
        val if val == FileBothDirectoryInformation => {
          buffer_ptr_offset_to_typed_ref::<FILE_BOTH_DIR_INFORMATION>(buffer, offset) as _
        }
        val if val == FileNamesInformation => {
          buffer_ptr_offset_to_typed_ref::<FILE_NAMES_INFORMATION>(buffer, offset) as _
        }
        _ => return filenames,
      };
      filenames.push(file_information.filename().to_string_lossy());
      let next_offset = file_information.offset();
      if next_offset != 0 {
        total_offset = Some(offset + next_offset)
      }
    }
    filenames
  }
}

trait FileInformationLike {
  fn filename(&self) -> widestring::U16CString;

  fn offset(&self) -> usize;
}

impl FileInformationLike for FILE_DIRECTORY_INFORMATION {
  fn filename(&self) -> widestring::U16CString {
    unsafe {
      widestring::U16CString::from_ptr_truncate(
        &raw const self.FileName[0],
        (self.FileNameLength / 2) as usize,
      )
    }
  }

  fn offset(&self) -> usize {
    self.NextEntryOffset as usize
  }
}

impl FileInformationLike for FILE_FULL_DIR_INFORMATION {
  fn filename(&self) -> widestring::U16CString {
    unsafe {
      widestring::U16CString::from_ptr_truncate(
        &raw const self.FileName[0],
        (self.FileNameLength / 2) as usize,
      )
    }
  }

  fn offset(&self) -> usize {
    self.NextEntryOffset as usize
  }
}

impl FileInformationLike for FILE_BOTH_DIR_INFORMATION {
  fn filename(&self) -> widestring::U16CString {
    unsafe {
      widestring::U16CString::from_ptr_truncate(
        &raw const self.FileName[0],
        (self.FileNameLength / 2) as usize,
      )
    }
  }

  fn offset(&self) -> usize {
    self.NextEntryOffset as usize
  }
}

impl FileInformationLike for FILE_NAMES_INFORMATION {
  fn filename(&self) -> widestring::U16CString {
    unsafe {
      widestring::U16CString::from_ptr_truncate(
        &raw const self.FileName[0],
        (self.FileNameLength / 2) as usize,
      )
    }
  }

  fn offset(&self) -> usize {
    self.NextEntryOffset as usize
  }
}
