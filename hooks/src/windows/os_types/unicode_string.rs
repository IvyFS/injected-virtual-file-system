use std::{ffi::OsStr, fmt::Debug, ops::Deref, path::Path};

use shared_types::HookError;
use win_api::{
  Wdk::Storage::FileSystem::RtlInitUnicodeStringEx, Win32::Foundation::UNICODE_STRING,
};
use win_types::PCWSTR;

pub struct ManagedUnicodeString {
  _str_owner: widestring::U16CString,
  str_ptr: *const UNICODE_STRING,
}

impl ManagedUnicodeString {
  pub fn from_path(
    path: impl AsRef<Path>,
    prefix: Option<impl AsRef<OsStr>>,
    suffix: Option<impl AsRef<OsStr>>,
  ) -> Result<Self, HookError> {
    let (str_owner, unicode) = unsafe { format_path_into_unicode_string(path, prefix, suffix)? };
    Ok(ManagedUnicodeString {
      _str_owner: str_owner,
      str_ptr: Box::into_raw(Box::new(unicode)),
    })
  }

  pub const fn nil_fix() -> Option<&'static str> {
    None
  }

  pub fn transact<T>(&self, func: impl FnOnce(*const UNICODE_STRING) -> T) -> T {
    func(self.str_ptr)
  }

  pub fn guard(&self) -> UnicodeStringGuard {
    UnicodeStringGuard(self)
  }
}

impl Debug for ManagedUnicodeString {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let unicode_buffer = unsafe {
      self
        .str_ptr
        .as_ref()
        .map(|u| u.Buffer.display().to_string())
    };
    f.debug_struct("ManagedUnicodeString")
      .field("_str_owner", &self._str_owner)
      .field("str_ptr", &unicode_buffer)
      .finish()
  }
}

pub struct UnicodeStringGuard<'a>(&'a ManagedUnicodeString);

impl<'a> Deref for UnicodeStringGuard<'a> {
  type Target = *const UNICODE_STRING;

  fn deref(&self) -> &Self::Target {
    &self.0.str_ptr
  }
}

impl Drop for ManagedUnicodeString {
  fn drop(&mut self) {
    unsafe { drop(Box::from_raw(self.str_ptr.cast_mut())) };
  }
}

pub unsafe fn path_to_unicode_string(
  path: impl AsRef<Path>,
) -> Result<(widestring::U16CString, UNICODE_STRING), HookError> {
  unsafe { format_path_into_unicode_string(path, Some("\\??\\"), Option::<String>::None) }
}

pub unsafe fn format_path_into_unicode_string(
  path: impl AsRef<Path>,
  prefix: Option<impl AsRef<OsStr>>,
  suffix: Option<impl AsRef<OsStr>>,
) -> Result<(widestring::U16CString, UNICODE_STRING), HookError> {
  let path = path.as_ref();
  unsafe {
    let mut unicode = UNICODE_STRING::default();
    let mut wide_str = prefix
      .map(|p| widestring::U16String::from_os_str(p.as_ref()))
      .unwrap_or_default();
    wide_str.push_os_str(path.as_os_str());
    if let Some(suffix) = suffix {
      wide_str.push_os_str(suffix);
    }
    let wide_str = widestring::U16CString::from_ustr(wide_str)
      .map_err(|_| HookError::UnicodeInit(path.as_os_str().to_owned()))?;
    let pcwstr = PCWSTR::from_raw(wide_str.as_ptr());
    let res = RtlInitUnicodeStringEx(&mut unicode, pcwstr);
    unicode.Length = unicode.Buffer.len() as u16;

    if res.is_err() {
      Err(HookError::UnicodeInit(path.as_os_str().to_owned()))
    } else {
      Ok((wide_str, unicode))
    }
  }
}
