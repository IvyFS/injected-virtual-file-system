use std::{ffi::OsStr, path::Path};

use proc_macros::ctest;
use win_api::Win32::{
  Foundation::ERROR_NO_MORE_FILES,
  Storage::FileSystem::{
    FIND_FIRST_EX_FLAGS, FindExInfoStandard, FindExSearchNameMatch, FindFirstFileExW,
    FindNextFileW, WIN32_FIND_DATAW,
  },
};
use windows_strings::PCWSTR;

use crate::common::{inject_self, workspace_root};

pub(crate) fn win32_find_files(filename: impl AsRef<OsStr>) -> Vec<widestring::U16CString> {
  let filename = widestring::U16CString::from_os_str_truncate(filename);
  let lpfilename = PCWSTR::from_raw(filename.as_ptr());
  let mut find_file_data = WIN32_FIND_DATAW::default();
  let handle = unsafe {
    FindFirstFileExW(
      lpfilename,
      FindExInfoStandard,
      &raw mut find_file_data as _,
      FindExSearchNameMatch,
      None,
      FIND_FIRST_EX_FLAGS::default(),
    )
    .unwrap()
  };

  assert!(!handle.is_invalid());

  let mut found_files = vec![
    widestring::U16CStr::from_slice_truncate(&find_file_data.cFileName)
      .unwrap()
      .to_ucstring(),
  ];

  loop {
    unsafe {
      find_file_data = Default::default();
      if let Err(err) = FindNextFileW(handle, &raw mut find_file_data as _) {
        if err.code() == win_api::core::HRESULT::from_win32(ERROR_NO_MORE_FILES.0) {
          break;
        } else {
          panic!("{err:?}")
        }
      }
      found_files.push(
        widestring::U16CStr::from_slice_truncate(&find_file_data.cFileName)
          .unwrap()
          .to_owned(),
      );
    }
  }
  found_files
}

#[ctest(super::TESTS)]
fn absolute_redirect() {
  let workspace_root = workspace_root();
  let virtual_root = workspace_root.join("integration\\target_folder");
  let mount_point = workspace_root.join("integration\\examples");

  inject_self(&virtual_root, &mount_point);

  let found_files = win32_find_files(mount_point.join("*"));

  for expected in vec![
    widestring::u16cstr!("."),
    widestring::u16cstr!(".."),
    widestring::u16cstr!("virtual_mod"),
    widestring::u16cstr!("enabled_mods.json"),
  ] {
    assert!(
      found_files.contains(&expected.to_ucstring()),
      "expected file {expected:?} not in found {found_files:?}"
    )
  }
  assert_eq!(found_files.len(), 4)
}

#[ctest(super::TESTS)]
fn relative_redirect() {
  let common_dir = Path::new("D:/Games/Starsector");
  std::env::set_current_dir(common_dir.join("starsector-core")).unwrap();

  let workspace_root = workspace_root();
  let virtual_root = workspace_root.join("integration\\target_folder");
  let mount_point = workspace_root.join(common_dir.join("mods"));

  inject_self(&virtual_root, &mount_point);

  let found_files = win32_find_files("../mods/*");

  for expected in vec![
    widestring::u16cstr!("."),
    widestring::u16cstr!(".."),
    widestring::u16cstr!("virtual_mod"),
    widestring::u16cstr!("enabled_mods.json"),
  ] {
    assert!(
      found_files.contains(&expected.to_ucstring()),
      "expected file {expected:?} not in found {found_files:?}"
    )
  }
  assert_eq!(found_files.len(), 4)
}

#[ctest(super::TESTS)]
fn no_redirect() {
  let workspace_root = workspace_root();
  let virtual_root = workspace_root.join("integration\\target_folder");
  let mount_point = workspace_root.join("integration\\examples");

  inject_self(&virtual_root, &mount_point);

  let found_files = win32_find_files(virtual_root.join("*"));

  for expected in vec![
    widestring::u16cstr!("."),
    widestring::u16cstr!(".."),
    widestring::u16cstr!("virtual_mod"),
    widestring::u16cstr!("enabled_mods.json"),
  ] {
    assert!(
      found_files.contains(&expected.to_ucstring()),
      "expected file {expected:?} not in found {found_files:?}"
    )
  }
  assert_eq!(found_files.len(), 4)
}
