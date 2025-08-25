use std::path::Path;

use integration_shared::TestHarness;
use proc_macros::ctest;
use win_api::{
  Wdk::{
    Foundation::OBJECT_ATTRIBUTES,
    Storage::FileSystem::{FILE_DIRECTORY_FILE, FILE_OPEN, NtCreateFile, RtlInitUnicodeStringEx},
    System::SystemServices::FILE_SHARE_VALID_FLAGS,
  },
  Win32::{
    Foundation::{HANDLE, UNICODE_STRING},
    Storage::FileSystem::{FILE_LIST_DIRECTORY, FILE_SHARE_MODE},
  },
};
use windows_strings::PCWSTR;

const NT_OPEN_CREATE_BIN: &str = env!("CARGO_BIN_EXE_NT_OPEN_CREATE");

#[ctest(crate::TESTS)]
fn open_existing_dir_in_virtual_fs() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN).parallel();

  std::fs::create_dir(&test_harness.virtual_expected()).unwrap();

  test_harness.set_args([
    "--is-dir".to_owned(),
    test_harness.mount_expected().display().to_string(),
  ]);

  assert!(test_harness.write_config_and_output().status.success())
}

#[ctest(crate::TESTS)]
fn mkdir_creates_dir_in_virtual_fs() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN).parallel();

  std::fs::create_dir(&test_harness.virtual_expected()).unwrap();

  test_harness.set_args([
    "--is-dir".to_owned(),
    test_harness.virtual_expected().display().to_string(),
  ]);

  assert!(test_harness.write_config_and_output().status.success())
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
