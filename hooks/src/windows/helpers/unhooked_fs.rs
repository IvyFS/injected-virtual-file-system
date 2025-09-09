use std::{
  borrow::Cow,
  ffi::OsString,
  os::windows::ffi::OsStringExt,
  path::{Path, PathBuf},
};

use shared_types::HookError;
use win_api::{
  Wdk::{
    Foundation::OBJECT_ATTRIBUTES,
    Storage::FileSystem::{FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE},
    System::SystemServices::FILE_SHARE_VALID_FLAGS,
  },
  Win32::{
    Foundation::{HANDLE, NTSTATUS, STATUS_NO_SUCH_FILE},
    Storage::FileSystem::{FILE_GENERIC_READ, FILE_LIST_DIRECTORY},
    System::IO::IO_STATUS_BLOCK,
  },
};
use win_types::{PCSTR, PCWSTR};

use crate::windows::{
  helpers::unicode_string::{OwnedUnicodeString, UnicodeError},
  patches::original_nt_open_file,
};

#[derive(Debug)]
pub enum UnhookedFsError {
  UnicodeError(UnicodeError),
  NoSuchFile,
  Other(NTSTATUS),
}

impl From<UnhookedFsError> for HookError {
  fn from(value: UnhookedFsError) -> Self {
    match value {
      UnhookedFsError::UnicodeError(unicode_error) => unicode_error.into(),
      UnhookedFsError::NoSuchFile => HookError::StdIO(std::io::ErrorKind::NotFound.into()),
      UnhookedFsError::Other(ntstatus) => {
        HookError::StdIO(std::io::Error::other(format!("{:x}", ntstatus.0)))
      }
    }
  }
}

pub struct PathLike<'a>(Cow<'a, Path>);

impl<'a> From<PCSTR> for PathLike<'a> {
  fn from(value: PCSTR) -> Self {
    let path = Path::new(unsafe { str::from_utf8_unchecked(value.as_bytes()) });
    PathLike(Cow::Owned(path.to_path_buf()))
  }
}

impl<'a> From<PCWSTR> for PathLike<'a> {
  fn from(value: PCWSTR) -> Self {
    let path = OsString::from_wide(unsafe { value.as_wide() });
    PathLike(Cow::Owned(path.into()))
  }
}

impl<'a> From<PathBuf> for PathLike<'a> {
  fn from(value: PathBuf) -> Self {
    Self(value.into())
  }
}

impl<'a> From<&'a Path> for PathLike<'a> {
  fn from(value: &'a Path) -> Self {
    Self(value.into())
  }
}

pub fn nt_open<'a>(path: impl Into<PathLike<'a>>, is_dir: bool) -> Result<HANDLE, UnhookedFsError> {
  let path = path.into();

  let mut handle = HANDLE::default();
  let object_attributes = create_obj_attributes(&path.0)?;
  let mut io_status_block = IO_STATUS_BLOCK::default();
  let (desired_access, open_options) = if is_dir {
    (FILE_LIST_DIRECTORY, FILE_DIRECTORY_FILE)
  } else {
    (FILE_GENERIC_READ, FILE_NON_DIRECTORY_FILE)
  };

  let status = unsafe {
    original_nt_open_file(
      &raw mut handle,
      desired_access.0,
      &raw const object_attributes.obj_attrs,
      &raw mut io_status_block,
      FILE_SHARE_VALID_FLAGS,
      open_options.0,
    )
  };

  match status {
    _ if status.is_ok() => Ok(handle),
    STATUS_NO_SUCH_FILE => Err(UnhookedFsError::NoSuchFile),
    status => Err(UnhookedFsError::Other(status)),
  }
}

struct OwnedObjAttributes {
  obj_attrs: OBJECT_ATTRIBUTES,
  _unicode_path: OwnedUnicodeString,
}

fn create_obj_attributes(
  absolute_path: impl AsRef<Path>,
) -> Result<OwnedObjAttributes, UnhookedFsError> {
  let unicode_path = unsafe { OwnedUnicodeString::path_to_unicode_nt_path(absolute_path) }
    .map_err(UnhookedFsError::UnicodeError)?;

  let mut obj_attrs = OBJECT_ATTRIBUTES::default();

  obj_attrs.Length = size_of::<OBJECT_ATTRIBUTES>() as u32;
  obj_attrs.ObjectName = unicode_path.unicode_ptr;
  Ok(OwnedObjAttributes {
    obj_attrs,
    _unicode_path: unicode_path,
  })
}

pub fn path_exists<'a>(
  path: impl Into<PathLike<'a>>,
  is_dir: bool,
) -> Result<bool, UnhookedFsError> {
  let res = nt_open(path, is_dir);

  match res {
    Ok(_) => Ok(true),
    Err(UnhookedFsError::NoSuchFile) => Ok(false),
    Err(err) => Err(err),
  }
}
