use proc_macros::ctest;
use win_api::{
  Wdk::{
    Storage::FileSystem::{
      FILE_DIRECTORY_INFORMATION, FileDirectoryInformation, NtQueryDirectoryFileEx,
    },
    System::SystemServices::SL_RETURN_SINGLE_ENTRY,
  },
  Win32::{
    Foundation::{HANDLE, STATUS_NO_MORE_FILES, STATUS_SUCCESS},
    System::IO::IO_STATUS_BLOCK,
  },
};

use crate::{
  common::{inject_self, workspace_root},
  nt_create::nt_create_open_existing_dir,
  nt_open::nt_open_existing_dir,
};

pub fn query_directory_file_all(handle: HANDLE) -> Vec<widestring::U16CString> {
  const BUF_LEN: usize = 1024;

  let mut res = Vec::new();
  loop {
    let mut io_status_block: IO_STATUS_BLOCK = Default::default();
    let mut file_infomation: [u8; BUF_LEN] = [0; BUF_LEN];
    unsafe {
      let (prefix, aligned, _suffix) = file_infomation.align_to_mut::<FILE_DIRECTORY_INFORMATION>();

      let status = NtQueryDirectoryFileEx(
        handle,
        None,
        None,
        None,
        &raw mut io_status_block,
        aligned.as_mut_ptr() as _,
        BUF_LEN as u32 - prefix.len() as u32,
        FileDirectoryInformation,
        SL_RETURN_SINGLE_ENTRY,
        None,
      );
      match status {
        STATUS_SUCCESS => {}
        STATUS_NO_MORE_FILES => break,
        _ => panic!("error: {:X}", status.0),
      }

      let info = &aligned[0];
      let filename = widestring::U16CString::from_ptr(
        &raw const info.FileName[0],
        (info.FileNameLength / 2) as usize,
      )
      .unwrap();
      res.push(filename.to_owned());
    }
  }

  res
}

#[ctest(super::TESTS)]
fn nt_create_query() {
  let workspace_root = workspace_root();
  let virtual_root = workspace_root.join("integration\\target_folder");
  let mount_point = workspace_root.join("integration\\examples");

  inject_self(&virtual_root, &mount_point);

  let handle = nt_create_open_existing_dir(&mount_point);
  let found = query_directory_file_all(handle);

  for expected in vec![
    widestring::u16cstr!("."),
    widestring::u16cstr!(".."),
    widestring::u16cstr!("virtual_mod"),
    widestring::u16cstr!("enabled_mods.json"),
  ] {
    assert!(
      found.contains(&expected.to_ucstring()),
      "expected file {expected:?} not in found {found:?}"
    )
  }
  assert_eq!(found.len(), 4)
}

#[ctest(super::TESTS)]
fn nt_open_query() {
  let workspace_root = workspace_root();
  let virtual_root = workspace_root.join("integration\\target_folder");
  let mount_point = workspace_root.join("integration\\examples");

  inject_self(&virtual_root, &mount_point);

  let handle = nt_open_existing_dir(&mount_point);
  let found = query_directory_file_all(handle);

  for expected in vec![
    widestring::u16cstr!("."),
    widestring::u16cstr!(".."),
    widestring::u16cstr!("virtual_mod"),
    widestring::u16cstr!("enabled_mods.json"),
  ] {
    assert!(
      found.contains(&expected.to_ucstring()),
      "expected file {expected:?} not in found {found:?}"
    )
  }
  assert_eq!(found.len(), 4)
}

// TODO: add restart test
