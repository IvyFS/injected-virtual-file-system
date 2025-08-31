use std::{
  ffi::{OsStr, OsString},
  fmt::Debug,
  os::windows::ffi::OsStringExt,
  path::Path,
};

use shared_types::HookError;
use widestring::U16CString;
use win_api::{
  Wdk::Storage::FileSystem::RtlInitUnicodeStringEx, Win32::Foundation::UNICODE_STRING,
};
use win_types::PCWSTR;

pub struct OwnedUnicodeString {
  pub string_buffer: widestring::U16CString,
  pub unicode_ptr: *const UNICODE_STRING,
}

impl OwnedUnicodeString {
  pub fn new(
    base: impl AsRef<OsStr>,
    prefix: Option<impl AsRef<OsStr>>,
    suffix: Option<impl AsRef<OsStr>>,
  ) -> Result<Self, HookError> {
    let (str_owner, unicode) = unsafe { format_unicode_string(base, prefix, suffix)? };
    Ok(OwnedUnicodeString {
      string_buffer: str_owner,
      unicode_ptr: Box::into_raw(Box::new(unicode)),
    })
  }

  pub unsafe fn path_to_unicode_nt_path(path: impl AsRef<Path>) -> Result<Self, HookError> {
    Self::new(path.as_ref(), Some("\\??\\"), nil_fix())
  }
}

impl Debug for OwnedUnicodeString {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("OwnedUnicodeString")
      .field("string_buffer", &self.string_buffer)
      .field("unicode_string len", &unsafe { *self.unicode_ptr }.Length)
      .finish()
  }
}

pub const fn nil_fix() -> Option<&'static str> {
  None
}

impl TryFrom<PCWSTR> for OwnedUnicodeString {
  type Error = HookError;

  fn try_from(pcwstr: PCWSTR) -> Result<OwnedUnicodeString, HookError> {
    let mut unicode = UNICODE_STRING::default();
    let res = unsafe { RtlInitUnicodeStringEx(&mut unicode, pcwstr) };

    if res.is_err() {
      Err(HookError::UnicodeInit(OsString::from_wide(unsafe {
        pcwstr.as_wide()
      })))
    } else {
      Ok(OwnedUnicodeString {
        string_buffer: Default::default(),
        unicode_ptr: Box::into_raw(Box::new(unicode)),
      })
    }
  }
}

impl TryFrom<&Path> for OwnedUnicodeString {
  type Error = HookError;

  fn try_from(value: &Path) -> Result<Self, Self::Error> {
    unsafe { OwnedUnicodeString::path_to_unicode_nt_path(value) }
  }
}

impl TryFrom<&OsStr> for OwnedUnicodeString {
  type Error = HookError;

  fn try_from(value: &OsStr) -> Result<Self, Self::Error> {
    OwnedUnicodeString::new(value, nil_fix(), nil_fix())
  }
}

impl TryFrom<U16CString> for OwnedUnicodeString {
  type Error = HookError;

  fn try_from(wide_str: U16CString) -> Result<Self, Self::Error> {
    let mut unicode = UNICODE_STRING::default();
    let pcwstr = PCWSTR::from_raw(wide_str.as_ptr());
    let res = unsafe { RtlInitUnicodeStringEx(&mut unicode, pcwstr) };

    if res.is_ok() {
      Ok(OwnedUnicodeString {
        string_buffer: wide_str,
        unicode_ptr: Box::into_raw(Box::new(unicode)),
      })
    } else {
      Err(HookError::UnicodeInit(wide_str.to_os_string()))
    }
  }
}

pub unsafe fn format_unicode_string(
  base: impl AsRef<OsStr>,
  prefix: Option<impl AsRef<OsStr>>,
  suffix: Option<impl AsRef<OsStr>>,
) -> Result<(widestring::U16CString, UNICODE_STRING), HookError> {
  let path = base.as_ref();
  unsafe {
    let mut unicode = UNICODE_STRING::default();
    let mut wide_str = prefix
      .map(|p| widestring::U16String::from_os_str(p.as_ref()))
      .unwrap_or_default();
    wide_str.push_os_str(path);
    if let Some(suffix) = suffix {
      wide_str.push_os_str(suffix);
    }
    let wide_str = widestring::U16CString::from_ustr(wide_str)
      .map_err(|_| HookError::UnicodeInit(path.to_owned()))?;
    let pcwstr = PCWSTR::from_raw(wide_str.as_ptr());
    let res = RtlInitUnicodeStringEx(&mut unicode, pcwstr);

    if res.is_err() {
      Err(HookError::UnicodeInit(path.to_owned()))
    } else {
      Ok((wide_str, unicode))
    }
  }
}

#[cfg(test)]
mod test {
  use crate::{
    raw_ptr::UnsafePtrCast,
    windows::helpers::unicode_string::{OwnedUnicodeString, nil_fix},
  };

  #[test]
  fn test_unicode_str_persists_after_init() {
    const PATH: &str = "C:\\some\\path";

    let owned_unicode = OwnedUnicodeString::new(PATH, nil_fix(), nil_fix()).unwrap();

    let unicode_inner = unsafe {
      owned_unicode
        .unicode_ptr
        .ref_cast()
        .unwrap()
        .Buffer
        .to_string()
        .unwrap()
    };
    assert_eq!(unicode_inner, PATH)
  }
}
