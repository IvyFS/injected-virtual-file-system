use std::{
  path::Path,
  ptr::{null, null_mut},
};

use integration_shared::{inject_self, workspace_root};
use proc_macros::ctest;
use win_api::{
  Wdk::{
    Foundation::OBJECT_ATTRIBUTES,
    Storage::FileSystem::{FILE_DIRECTORY_FILE, NtOpenFile, RtlInitUnicodeStringEx},
    System::SystemServices::FILE_SHARE_VALID_FLAGS,
  },
  Win32::{
    Foundation::{HANDLE, OBJECT_ATTRIBUTE_FLAGS, UNICODE_STRING},
    Storage::FileSystem::FILE_LIST_DIRECTORY,
    System::IO::IO_STATUS_BLOCK,
  },
};
use windows_strings::PCWSTR;

use crate::common::path_from_handle;

#[ctest(crate::TESTS)]
fn test_dir() {


  let workspace_root = workspace_root();
  let virtual_root = workspace_root.join("integration\\target_folder");
  let mount_point = workspace_root.join("integration\\examples");

  inject_self(&virtual_root, &mount_point);

  unsafe {
    let filehandle = nt_open_existing_dir(&mount_point);

    assert_eq!(
      "\\\\?\\C:\\Users\\wanty\\Documents\\usvfs-rust\\integration\\target_folder",
      path_from_handle(filehandle)
    );
  }
}

pub(crate) fn nt_open_existing_dir(path: &Path) -> HANDLE {
  let mut filehandle = HANDLE::default();

  let mut raw_object_name = widestring::U16String::from_os_str("\\??\\");
  raw_object_name.push_os_str(&path);
  let raw_object_name = widestring::U16CString::from_ustr(raw_object_name).unwrap();
  let pcw_object_name = PCWSTR::from_raw(raw_object_name.as_ptr());
  let mut object_name = UNICODE_STRING::default();
  unsafe {
    assert!(RtlInitUnicodeStringEx(&raw mut object_name, pcw_object_name).is_ok());
  }

  let objectattributes = OBJECT_ATTRIBUTES {
    Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
    RootDirectory: HANDLE(null_mut()),
    ObjectName: &raw const object_name,
    Attributes: OBJECT_ATTRIBUTE_FLAGS::default(),
    SecurityDescriptor: null(),
    SecurityQualityOfService: null(),
  };

  let mut iostatusblock = IO_STATUS_BLOCK::default();

  let status = unsafe {
    NtOpenFile(
      &raw mut filehandle,
      FILE_LIST_DIRECTORY.0,
      &raw const objectattributes,
      &raw mut iostatusblock,
      FILE_SHARE_VALID_FLAGS,
      FILE_DIRECTORY_FILE.0,
    )
  };
  assert!(status.is_ok(), "{:x}", status.0);
  filehandle
}
