use std::{
  collections::HashMap,
  ffi::c_void,
  hash::Hash,
  ops::Deref,
  sync::{LazyLock, Mutex, MutexGuard},
};

use ref_cast::RefCast;
use shared_types::HookError;
use win_api::{
  Wdk::Foundation::OBJECT_ATTRIBUTES,
  Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{GETFINALPATHNAMEBYHANDLE_FLAGS, GetFinalPathNameByHandleW},
  },
};

use crate::raw_ptr::UnsafeRefCast;

#[derive(Clone, Copy, Debug, PartialEq, Eq, RefCast)]
#[repr(transparent)]
pub struct Handle(HANDLE);

impl From<HANDLE> for Handle {
  fn from(value: HANDLE) -> Self {
    Self(value)
  }
}

impl From<*mut c_void> for Handle {
  fn from(value: *mut c_void) -> Self {
    Self(HANDLE(value))
  }
}

impl Deref for Handle {
  type Target = HANDLE;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> AsRef<T> for Handle
where
  T: ?Sized,
  <Handle as Deref>::Target: AsRef<T>,
{
  fn as_ref(&self) -> &T {
    self.deref().as_ref()
  }
}

impl Hash for Handle {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.0.0.hash(state);
  }
}

/// # Safety
///
/// The Win32 API makes no indication that file handles cannot be used across threads.
/// In fact, the prevailing advice is that all Win32 types are thread-safe unless stated otherwise.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

pub static HANDLE_MAP: LazyLock<Mutex<HashMap<Handle, HandleInfo>>> =
  LazyLock::new(|| Default::default());

pub struct HandleInfo {
  pub path: String,
}

pub struct HandleMap;

impl HandleMap {
  pub fn get<'a>() -> Result<MutexGuard<'a, HashMap<Handle, HandleInfo>>, HookError> {
    Ok(HANDLE_MAP.lock()?)
  }

  pub unsafe fn update_handles(
    handle: impl Into<Handle>,
    attrs: impl UnsafeRefCast<OBJECT_ATTRIBUTES>,
  ) -> Result<(), HookError> {
    unsafe {
      let attrs = attrs.ref_cast()?;

      let path = attrs.path()?;

      let mut handle_map = Self::get()?;
      handle_map
        .entry(handle.into())
        .or_insert_with(|| HandleInfo { path });
    }
    Ok(())
  }
}

pub trait ObjectAttributesExt {
  unsafe fn path(&self) -> Result<String, HookError>;
}

impl ObjectAttributesExt for OBJECT_ATTRIBUTES {
  unsafe fn path(&self) -> Result<String, HookError> {
    unsafe {
      let stem = self.ObjectName.ref_cast()?.Buffer.to_string()?;

      let seen_handles = HANDLE_MAP.lock()?;
      let handle = Handle::ref_cast(&self.RootDirectory);
      if let Some(info) = seen_handles.get(handle) {
        Ok(format!("{}\\{stem}", info.path))
      } else if let Ok(path) = path_from_handle(handle) {
        Ok(path)
      } else {
        Ok(stem)
      }
    }
  }
}

pub unsafe fn path_from_handle(handle: &HANDLE) -> Result<String, HookError> {
  unsafe {
    let mut buffer = [0; 1024];
    let len = GetFinalPathNameByHandleW(
      handle.clone(),
      &mut buffer,
      GETFINALPATHNAMEBYHANDLE_FLAGS::default(),
    );
    if len > 0 {
      Ok(String::from_utf16(&buffer[0..(len as usize)])?)
    } else {
      Err(HookError::PathFromFileHandle)
    }
  }
}
