use std::{ffi::OsStr, path::PathBuf};

use clap::Parser;
use integration_shared::output::write_output;
use win_api::Win32::{
  Foundation::ERROR_NO_MORE_FILES,
  Storage::FileSystem::{
    FIND_FIRST_EX_FLAGS, FindExInfoStandard, FindExSearchNameMatch, FindFirstFileExW,
    FindNextFileW, WIN32_FIND_DATAW,
  },
};
use windows_strings::PCWSTR;

#[derive(Debug, Parser)]
struct Args {
  path: PathBuf,
  output: PathBuf,
}

fn main() {
  let Args { path, output } = Args::parse();

  let results: Vec<_> = win32_find_files(path)
    .into_iter()
    .map(|filename| filename.to_string().unwrap())
    .collect();

  write_output(results, output);
}

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
