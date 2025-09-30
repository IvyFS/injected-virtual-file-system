use std::{
  os::windows::{fs::OpenOptionsExt, io::IntoRawHandle},
  path::Path,
};

use widestring::U16CString;
use win_api::{
  Wdk::{
    Storage::FileSystem::{
      FILE_DIRECTORY_INFORMATION, FileDirectoryInformation, NtQueryDirectoryFileEx,
    },
    System::SystemServices::SL_RETURN_SINGLE_ENTRY,
  },
  Win32::{
    Foundation::{HANDLE, STATUS_NO_MORE_FILES, STATUS_SUCCESS},
    Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS,
    System::IO::IO_STATUS_BLOCK,
  },
};

fn main() {
  let mut args = std::env::args().skip(1);

  let search_dir_path = args.next().unwrap();
  assert!(Path::new(&search_dir_path).is_dir(), "{search_dir_path} is not a directory");
  let expected: Vec<U16CString> = args
    .map(|str| U16CString::from_str_truncate(&str))
    .collect();

  let search_dir = std::fs::File::options()
    .read(true)
    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
    .open(search_dir_path)
    .unwrap()
    .into_raw_handle();

  let dir_contents = query_directory_file_all(HANDLE(search_dir));

  assert_eq!(expected.len(), dir_contents.len());
  for found in dir_contents {
    assert!(expected.contains(&found), "{found:#?} not in expected list {expected:#?}")
  }
}

pub fn query_directory_file_all(handle: HANDLE) -> Vec<U16CString> {
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
