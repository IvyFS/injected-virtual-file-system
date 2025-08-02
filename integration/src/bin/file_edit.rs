use std::ffi::CString;

use clap::{Parser, Subcommand};
use widestring::U16CString;
use win_api::Win32::Storage::FileSystem::{DeleteFileW, MoveFileA, MoveFileW};
use windows_strings::{PCSTR, PCWSTR};

#[derive(Debug, Parser)]
struct Cli {
  #[command(subcommand)]
  subcommand: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  Delete { path: String },
  MoveFileA { source: String, dest: String },
  MoveFileW { source: String, dest: String },
}

fn main() {
  std::thread::sleep(std::time::Duration::from_millis(500));

  match Cli::parse_from(dbg!(std::env::args())).subcommand {
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
