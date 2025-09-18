use std::{
  borrow::Cow,
  ffi::OsString,
  io::ErrorKind,
  os::windows::ffi::OsStringExt,
  path::{Path, PathBuf},
  ptr::{null, null_mut},
};

use shared_types::HookError;
use win_api::{
  Wdk::{
    Foundation::OBJECT_ATTRIBUTES,
    Storage::FileSystem::{FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE},
    System::SystemServices::FILE_SHARE_VALID_FLAGS,
  },
  Win32::{
    Foundation::{HANDLE, NTSTATUS, OBJECT_ATTRIBUTE_FLAGS, STATUS_OBJECT_NAME_NOT_FOUND},
    Storage::FileSystem::{FILE_GENERIC_READ, FILE_LIST_DIRECTORY},
    System::IO::IO_STATUS_BLOCK,
  },
};
use win_types::{PCSTR, PCWSTR};

use crate::windows::{
  helpers::unicode_string::{OwnedUnicodeString, UnicodeError},
  patches::original_nt_open_file,
};

pub enum UnhookedFsError {
  UnicodeError(UnicodeError),
  OS {
    nt_status: NTSTATUS,
    std: std::io::Error,
  },
}

impl std::fmt::Debug for UnhookedFsError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::UnicodeError(arg0) => f.debug_tuple("UnicodeError").field(arg0).finish(),
      Self::OS { nt_status, std } => f
        .debug_struct("OS")
        .field("nt_status", &format_args!("{:x}", nt_status.0))
        .field("std", std)
        .finish(),
    }
  }
}

impl From<NTSTATUS> for UnhookedFsError {
  fn from(value: NTSTATUS) -> Self {
    Self::OS {
      nt_status: value,
      std: std::io::Error::from_raw_os_error(value.0),
    }
  }
}

impl From<UnhookedFsError> for HookError {
  fn from(value: UnhookedFsError) -> Self {
    match value {
      UnhookedFsError::UnicodeError(unicode_error) => unicode_error.into(),
      UnhookedFsError::OS { std, .. } => HookError::StdIo(std),
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

struct OwnedObjectAttributes {
  _unicode_path: OwnedUnicodeString,
  obj_attrs: OBJECT_ATTRIBUTES,
}

impl OwnedObjectAttributes {
  fn new<'a>(path: impl Into<PathLike<'a>>) -> Result<Self, UnhookedFsError> {
    let path = path.into();

    let unicode_path = unsafe { OwnedUnicodeString::path_to_unicode_nt_path(&path.0) }
      .map_err(UnhookedFsError::UnicodeError)?;
    let obj_attrs = OBJECT_ATTRIBUTES {
      Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
      RootDirectory: HANDLE(null_mut()),
      ObjectName: unicode_path.unicode_ptr,
      Attributes: OBJECT_ATTRIBUTE_FLAGS::default(),
      SecurityDescriptor: null(),
      SecurityQualityOfService: null(),
    };

    Ok(Self {
      _unicode_path: unicode_path,
      obj_attrs,
    })
  }
}

pub fn nt_open_by_path<'a>(
  path: impl Into<PathLike<'a>>,
  is_dir: bool,
) -> Result<HANDLE, UnhookedFsError> {
  let obj_attrs = OwnedObjectAttributes::new(path)?;

  nt_open(&obj_attrs.obj_attrs, is_dir).map_err(Into::into)
}

pub fn nt_open(obj_attrs: &OBJECT_ATTRIBUTES, is_dir: bool) -> Result<HANDLE, NTSTATUS> {
  let mut handle = HANDLE::default();
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
      obj_attrs,
      &raw mut io_status_block,
      FILE_SHARE_VALID_FLAGS,
      open_options.0,
    )
  };

  if status.is_ok() {
    Ok(handle)
  } else {
    Err(status)
  }
}

pub fn exists_by_path<'a>(
  path: impl Into<PathLike<'a>>,
  is_dir: bool,
) -> Result<bool, UnhookedFsError> {
  let obj_attrs = OwnedObjectAttributes::new(path)?;

  exists(&obj_attrs.obj_attrs, is_dir).map_err(Into::into)
}

pub fn exists(obj_attrs: &OBJECT_ATTRIBUTES, is_dir: bool) -> Result<bool, NTSTATUS> {
  match nt_open(&obj_attrs, is_dir) {
    Ok(_) => Ok(true),
    Err(err)
      if err == STATUS_OBJECT_NAME_NOT_FOUND
        || std::io::Error::from_raw_os_error(err.0).kind() == ErrorKind::NotFound =>
    {
      Ok(false)
    }
    Err(err) => Err(err),
  }
}
