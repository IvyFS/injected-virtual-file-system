use std::ffi::CString;

use clap::Parser;
use widestring::U16CString;
use win_api::Win32::Storage::FileSystem::{DeleteFileW, MoveFileA, MoveFileW};
use windows_strings::{PCSTR, PCWSTR};

#[derive(Debug, Parser)]
#[clap(disable_help_flag = true, disable_help_subcommand = true)]
enum Command {
  Delete { path: String },
  MoveFileA { source: String, dest: String },
  MoveFileW { source: String, dest: String },
}

fn main() {
  match Command::parse() {
    Command::Delete { path } => unsafe {
      let filename = U16CString::from_os_str_truncate(path);
      DeleteFileW(PCWSTR::from_raw(filename.as_ptr())).unwrap();
    },
    Command::MoveFileA { source, dest } => unsafe {
      let source = CString::new(source.as_bytes()).unwrap();
      let dest = CString::new(dest.as_bytes()).unwrap();
      MoveFileA(
        PCSTR::from_raw(source.as_ptr() as _),
        PCSTR::from_raw(dest.as_ptr() as _),
      )
      .unwrap();
    },
    Command::MoveFileW { source, dest } => unsafe {
      let source = U16CString::from_os_str_truncate(source);
      let dest = U16CString::from_os_str_truncate(dest);
      MoveFileW(
        PCWSTR::from_raw(source.as_ptr()),
        PCWSTR::from_raw(dest.as_ptr()),
      )
      .unwrap();
    },
  }
}
