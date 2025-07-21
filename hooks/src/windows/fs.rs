use std::path::Path;

use shared_types::HookError;
use win_api::{
  Wdk::{
    Foundation::OBJECT_ATTRIBUTES,
    Storage::FileSystem::{
      FILE_DIRECTORY_FILE, FILE_INFORMATION_CLASS, FILE_OPEN_FOR_BACKUP_INTENT,
      FILE_SYNCHRONOUS_IO_NONALERT, NtOpenFile, NtQueryDirectoryFileEx,
    },
    System::SystemServices::SL_RETURN_SINGLE_ENTRY,
  },
  Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{
      FILE_LIST_DIRECTORY, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, SYNCHRONIZE,
    },
  },
};

use crate::windows::os_types::unicode_string::OwnedUnicodeString;

pub(crate) unsafe fn open_existing_dir(
  path: impl TryInto<OwnedUnicodeString, Error = HookError>,
) -> Result<HANDLE, HookError> {
  let mut filehandle = HANDLE::default();

  let object_name: OwnedUnicodeString = path.try_into()?;

  let objectattributes = OBJECT_ATTRIBUTES {
    Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
    RootDirectory: HANDLE(std::ptr::null_mut()),
    ObjectName: object_name.unicode_ptr,
    Attributes: Default::default(),
    SecurityDescriptor: std::ptr::null(),
    SecurityQualityOfService: std::ptr::null(),
  };

  let mut iostatusblock = Default::default();

  let status = unsafe {
    NtOpenFile(
      &raw mut filehandle,
      (FILE_LIST_DIRECTORY & SYNCHRONIZE).0,
      &raw const objectattributes,
      &raw mut iostatusblock,
      (FILE_SHARE_READ & FILE_SHARE_WRITE & FILE_SHARE_DELETE).0,
      (FILE_DIRECTORY_FILE & FILE_SYNCHRONOUS_IO_NONALERT & FILE_OPEN_FOR_BACKUP_INTENT).0,
    )
  };
  assert!(status.is_ok(), "{:x}", status.0);
  Ok(filehandle)
}

pub(crate) unsafe fn query_directory_file_single<
  INFO,
  FILT: TryInto<OwnedUnicodeString, Error = HookError>,
  OUT,
>(
  handle: HANDLE,
  file_information_class: FILE_INFORMATION_CLASS,
  filter: Option<FILT>,
  cb: impl FnOnce(&[INFO]) -> OUT,
) -> Result<OUT, HookError> {
  const BUF_LEN: usize = 1024;

  let filter: Option<OwnedUnicodeString> = filter.map(|f| f.try_into()).transpose()?;

  let mut io_status_block = Default::default();
  let mut buffer: [u8; BUF_LEN] = [0; BUF_LEN];
  unsafe {
    let (prefix, aligned, _) = buffer.align_to_mut::<INFO>();

    let res = NtQueryDirectoryFileEx(
      handle,
      None,
      None,
      None,
      &raw mut io_status_block,
      aligned.as_mut_ptr() as _,
      BUF_LEN as u32 - prefix.len() as u32,
      file_information_class,
      SL_RETURN_SINGLE_ENTRY,
      filter.as_ref().map(|f| f.unicode_ptr),
    );

    if res.is_ok() {
      Ok(cb(aligned))
    } else {
      Err(HookError::Other(res.to_hresult().to_string()))
    }
  }
}
