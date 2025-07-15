use std::{ffi::OsStr, fmt::Debug, ops::Deref, path::Path};

use shared_types::HookError;
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

  pub fn transact<T>(&self, func: impl FnOnce(*const UNICODE_STRING) -> T) -> T {
    func(self.unicode_ptr)
  }

  pub fn as_unicode_str(&'_ self) -> UnicodeStringGuard<'_> {
    UnicodeStringGuard(self)
  }
}

impl Debug for OwnedUnicodeString {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let unicode_buffer = unsafe {
      self
        .unicode_ptr
        .as_ref()
        .map(|u| u.Buffer.display().to_string())
    };
    f.debug_struct("ManagedUnicodeString")
      .field("_str_owner", &self.string_buffer)
      .field("str_ptr", &unicode_buffer)
      .finish()
  }
}

pub const fn nil_fix() -> Option<&'static str> {
  None
}

pub struct UnicodeStringGuard<'a>(&'a OwnedUnicodeString);

impl<'a> Deref for UnicodeStringGuard<'a> {
  type Target = *const UNICODE_STRING;

  fn deref(&self) -> &Self::Target {
    &self.0.unicode_ptr
  }
}

impl Drop for OwnedUnicodeString {
  fn drop(&mut self) {
    unsafe { drop(Box::from_raw(self.unicode_ptr.cast_mut())) };
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
    raw_ptr::UnsafeRefCast,
    windows::os_types::unicode_string::{OwnedUnicodeString, nil_fix},
  };

  #[test]
  fn test_unicode_str_persists_after_init() {
    const PATH: &str = "C:\\some\\path";

    let owned_unicode = OwnedUnicodeString::new(PATH, nil_fix(), nil_fix()).unwrap();

    let unicode_inner = unsafe {
      owned_unicode
        .as_unicode_str()
        .ref_cast()
        .unwrap()
        .Buffer
        .to_string()
        .unwrap()
    };
    assert_eq!(unicode_inner, PATH)
  }
}
