use std::{
  path::{Path, PathBuf},
  ptr::{null, null_mut},
};

use clap::{Parser, Subcommand};
use win_api::{
  Wdk::{
    Foundation::OBJECT_ATTRIBUTES,
    Storage::FileSystem::{
      FILE_CREATE, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_OVERWRITE,
      NtCreateFile, NtOpenFile, RtlInitUnicodeStringEx,
    },
    System::SystemServices::FILE_SHARE_VALID_FLAGS,
  },
  Win32::{
    Foundation::{HANDLE, OBJECT_ATTRIBUTE_FLAGS, UNICODE_STRING},
    Storage::FileSystem::{
      DELETE, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_LIST_DIRECTORY, FILE_SHARE_MODE,
    },
    System::IO::IO_STATUS_BLOCK,
  },
};
use windows_strings::PCWSTR;

#[derive(Debug, Parser)]
#[clap(
  disable_help_flag = true,
  disable_help_subcommand = true,
  subcommand_required = true
)]
struct Args {
  #[command(subcommand)]
  variant: Variant,
  path: PathBuf,
  #[arg(long, default_value_t = false)]
  is_dir: bool,
}

#[derive(Debug, Subcommand)]
enum Variant {
  Open,
  Create {
    #[arg(long, default_value_t = false)]
    create_not_exists: bool,
    #[arg(long, default_value_t = false)]
    truncate: bool,
  },
}

fn main() {
  let Args {
    variant,
    path,
    is_dir,
  } = Args::parse();

  match variant {
    Variant::Open => nt_open(&path, is_dir),
    Variant::Create {
      create_not_exists,
      truncate,
    } => {
      let open_options = match (create_not_exists, truncate) {
        (false, false) => OpenOptions::MustExist,
        (true, false) => OpenOptions::CreateMustNotExist,
        (false, true) => OpenOptions::TruncateMustExist,
        _ => unimplemented!(),
      };

      nt_create(&path, is_dir, open_options)
    }
  };
}

pub(crate) fn nt_open(path: &Path, is_dir: bool) -> HANDLE {
  let mut filehandle = HANDLE::default();

  let mut raw_object_name = widestring::U16String::from_os_str("\\??\\");
  raw_object_name.push_os_str(&path);
  let raw_object_name = dbg!(widestring::U16CString::from_ustr(raw_object_name).unwrap());
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

  let (desired_access, open_options) = if is_dir {
    (FILE_LIST_DIRECTORY, FILE_DIRECTORY_FILE)
  } else {
    (FILE_GENERIC_READ, FILE_NON_DIRECTORY_FILE)
  };

  let status = unsafe {
    NtOpenFile(
      &raw mut filehandle,
      desired_access.0,
      &raw const objectattributes,
      &raw mut iostatusblock,
      FILE_SHARE_VALID_FLAGS,
      open_options.0,
    )
  };
  assert!(status.is_ok(), "{:x}", status.0);
  filehandle
}

enum OpenOptions {
  MustExist,
  TruncateMustExist,
  CreateMustNotExist,
}

pub(crate) fn nt_create(path: &Path, is_dir: bool, open_options: OpenOptions) -> HANDLE {
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

  let (desired_access, create_options) = if is_dir {
    (FILE_LIST_DIRECTORY, FILE_DIRECTORY_FILE)
  } else {
    (
      FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE,
      FILE_NON_DIRECTORY_FILE,
    )
  };

  let status = unsafe {
    NtCreateFile(
      &raw mut filehandle,
      desired_access,
      &raw const objectattributes,
      &raw mut iostatusblock,
      None,
      Default::default(),
      FILE_SHARE_MODE(FILE_SHARE_VALID_FLAGS),
      match open_options {
        OpenOptions::MustExist => FILE_OPEN,
        OpenOptions::TruncateMustExist => FILE_OVERWRITE,
        OpenOptions::CreateMustNotExist => FILE_CREATE,
      },
      create_options,
      None,
      0,
    )
  };
  assert!(status.is_ok(), "{:x}", status.0);
  filehandle
}
