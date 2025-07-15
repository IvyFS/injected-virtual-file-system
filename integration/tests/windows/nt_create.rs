use std::{
  os::windows::{fs::OpenOptionsExt, io::IntoRawHandle},
  path::Path,
};

use win_api::{
  Wdk::{
    Foundation::OBJECT_ATTRIBUTES,
    Storage::FileSystem::{FILE_DIRECTORY_FILE, FILE_OPEN, NtCreateFile, RtlInitUnicodeStringEx},
    System::SystemServices::FILE_SHARE_VALID_FLAGS,
  },
  Win32::{
    Foundation::{HANDLE, UNICODE_STRING},
    Storage::FileSystem::{FILE_FLAG_BACKUP_SEMANTICS, FILE_LIST_DIRECTORY, FILE_SHARE_MODE},
  },
};
use windows_strings::PCWSTR;

use crate::common::{inject_self, path_from_handle, workspace_root};

#[test]
fn create_open_dir_std() {
  let workspace_root = workspace_root();
  let virtual_root = workspace_root.join("integration\\target_folder");
  let mount_point = workspace_root.join("integration\\examples");

  inject_self(&virtual_root, &mount_point);

  let handle = std::fs::File::options()
    .read(true)
    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
    .open(&mount_point)
    .unwrap()
    .into_raw_handle();

  assert_eq!(
    "\\\\?\\C:\\Users\\wanty\\Documents\\usvfs-rust\\integration\\target_folder",
    unsafe { path_from_handle(HANDLE(handle)) }
  );
}

#[test]
fn create_open_dir_manual() {
  let workspace_root = workspace_root();
  let virtual_root = workspace_root.join("integration\\target_folder");
  let mount_point = workspace_root.join("integration\\examples");

  inject_self(&virtual_root, mount_point.join("*"));

  unsafe {
    let filehandle = nt_create_open_existing_dir(&mount_point);

    assert_eq!(
      "\\\\?\\C:\\Users\\wanty\\Documents\\usvfs-rust\\integration\\target_folder",
      path_from_handle(filehandle)
    );
  }
}

pub(crate) fn nt_create_open_existing_dir(path: &Path) -> HANDLE {
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
    RootDirectory: HANDLE(std::ptr::null_mut()),
    ObjectName: &raw const object_name,
    Attributes: Default::default(),
    SecurityDescriptor: std::ptr::null(),
    SecurityQualityOfService: std::ptr::null(),
  };

  let mut iostatusblock = Default::default();

  let status = unsafe {
    NtCreateFile(
      &raw mut filehandle,
      FILE_LIST_DIRECTORY,
      &raw const objectattributes,
      &raw mut iostatusblock,
      None,
      Default::default(),
      FILE_SHARE_MODE(FILE_SHARE_VALID_FLAGS),
      FILE_OPEN,
      FILE_DIRECTORY_FILE,
      None,
      0,
    )
  };
  assert!(status.is_ok(), "{:x}", status.0);
  filehandle
}
