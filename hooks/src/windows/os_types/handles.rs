use std::{
  ffi::{OsString, c_void},
  hash::Hash,
  ops::Deref,
  os::windows::{ffi::OsStringExt, fs::OpenOptionsExt, io::IntoRawHandle},
  path::{Path, PathBuf},
  sync::{Arc, LazyLock},
};

use dashmap::DashMap;
use ref_cast::RefCast;
use shared_types::{ErrorContext, HookError};
use win_api::{
  Wdk::Foundation::OBJECT_ATTRIBUTES,
  Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{
      FILE_ATTRIBUTE_OFFLINE, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAGS_AND_ATTRIBUTES,
      GETFINALPATHNAMEBYHANDLE_FLAGS, GetFinalPathNameByHandleW,
    },
  },
};

use crate::{
  extension_traits::DashExt,
  log::logfmt_dbg,
  raw_ptr::UnsafeRefCast,
  virtual_paths::{MOUNT_POINT, VIRTUAL_ROOT, windows::VirtualPath},
  windows::os_types::paths::strip_nt_prefix,
};

#[allow(dead_code)]
pub const NULL_HANDLE: HANDLE = HANDLE(std::ptr::null_mut());

pub const DO_NOT_HOOK: FILE_FLAGS_AND_ATTRIBUTES = FILE_ATTRIBUTE_OFFLINE;

#[derive(Clone, Copy, Debug, PartialEq, Eq, RefCast)]
#[repr(transparent)]
pub struct Handle(HANDLE);

impl From<HANDLE> for Handle {
  fn from(value: HANDLE) -> Self {
    Self(value)
  }
}

impl From<Handle> for HANDLE {
  fn from(value: Handle) -> Self {
    value.0
  }
}

impl From<*mut c_void> for Handle {
  fn from(value: *mut c_void) -> Self {
    Self(HANDLE(value))
  }
}

impl From<*mut HANDLE> for Handle {
  fn from(value: *mut HANDLE) -> Self {
    Self(unsafe { value.read() })
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

pub static HANDLE_MAP: LazyLock<HandleMap> = LazyLock::new(Default::default);

pub fn std_open_dir_handle_unhooked(path: impl AsRef<Path>) -> Result<Handle, HookError> {
  let handle = std::fs::File::options()
    .read(true)
    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
    // .share_mode(DEFAULT_SHARE_MODE | DO_NOT_HOOK.0)
    .attributes(DO_NOT_HOOK.0)
    .open(path.as_ref())
    .with_context(|| format!("path = {:?}", path.as_ref()))?;
  Ok(handle.into_raw_handle().into())
}

#[derive(Debug, Clone)]
pub struct HandleInfo {
  pub path: PathBuf,
  pub handle: Handle,
  pub rerouted: bool,
}

#[derive(Debug, Default)]
pub struct HandleMap {
  ptr_keyed: DashMap<Handle, Arc<HandleInfo>>,
  path_keyed: DashMap<PathBuf, Arc<HandleInfo>>,
}

impl HandleMap {
  pub fn insert(&self, handle: impl Into<Handle>, path: impl AsRef<Path>, rerouted: bool) -> bool {
    let handle = handle.into();
    let path = strip_nt_prefix(&path).to_owned();
    let handle_info = Arc::new(HandleInfo {
      path: path.clone(),
      handle,
      rerouted,
    });
    self.ptr_keyed.try_insert(handle, Arc::clone(&handle_info))
      && self.path_keyed.try_insert(path, handle_info)
  }

  pub fn get_by_handle(
    &self,
    handle: impl Into<Handle>,
  ) -> Option<dashmap::mapref::one::Ref<'_, Handle, Arc<HandleInfo>>> {
    self.ptr_keyed.get(&handle.into())
  }

  pub fn remove_by_handle(&self, handle: impl Into<Handle>) -> Option<(Handle, Arc<HandleInfo>)> {
    let key = handle.into();
    let res = self.ptr_keyed.remove(&key);
    if let Some((_, info)) = res.as_ref() {
      self.path_keyed.remove(&info.path);
    }

    res
  }
}

pub trait ObjectAttributesExt {
  unsafe fn path(&self) -> Result<PathBuf, HookError>;
}

impl ObjectAttributesExt for OBJECT_ATTRIBUTES {
  unsafe fn path(&self) -> Result<PathBuf, HookError> {
    unsafe {
      let unicode_string = self.ObjectName.ref_cast()?;
      let stem_raw =
        OsString::from_wide(&unicode_string.Buffer.as_wide()[..(unicode_string.Length / 2) as usize]);
      let stem: &Path = stem_raw.as_ref();

      // TODO?: canonicalize but preserve nt prefix?

      let mut path = stem.to_owned();
      if !self.RootDirectory.is_invalid() {
        let handle = Handle::ref_cast(&self.RootDirectory);
        if let Some(info) = HANDLE_MAP.get_by_handle(*handle) {
          path = info.path.join(path);
        } else if let Ok(handle_path) = path_from_handle(handle) {
          path = Path::new(&handle_path).join(path);
        }
      }

      Ok(path)
    }
  }
}

impl<T: UnsafeRefCast<OBJECT_ATTRIBUTES> + Copy> ObjectAttributesExt for T {
  unsafe fn path(&self) -> Result<PathBuf, HookError> {
    unsafe { self.ref_cast()?.path() }
  }
}

pub unsafe fn path_from_handle(handle: &HANDLE) -> Result<String, HookError> {
  unsafe {
    const LEN: usize = 1024;
    let mut buffer = [0; LEN];
    let len = GetFinalPathNameByHandleW(
      *handle,
      &mut buffer,
      GETFINALPATHNAMEBYHANDLE_FLAGS::default(),
    );
    if len != 0 && len < LEN as u32 {
      Ok(String::from_utf16(&buffer[0..(len as usize)])?)
    } else {
      Err(HookError::PathFromFileHandle)
    }
  }
}

pub fn get_virtual_path(path: impl AsRef<Path>) -> Result<Option<VirtualPath>, HookError> {
  let trimmed = strip_nt_prefix(&path);
  let canon = dunce::simplified(trimmed).to_path_buf();

  match canon.strip_prefix(MOUNT_POINT.read()?.as_path()) {
    Ok(stem) => {
      let virtual_root = VIRTUAL_ROOT.read()?;
      let rerouted_path = if !stem.as_os_str().is_empty() {
        virtual_root.join(stem)
      } else {
        virtual_root.to_path_buf()
      };
      Ok(Some(VirtualPath {
        path: rerouted_path,
        original: canon.to_path_buf(),
      }))
    }
    _ => Ok(None),
  }
}

macro_rules! into_handle {
  () => {impl Into<crate::windows::os_types::handles::Handle>};
}

pub(crate) use into_handle;
