use std::{
  borrow::Borrow,
  ffi::{OsString, c_void},
  hash::Hash,
  ops::Deref,
  os::windows::{ffi::OsStringExt, fs::OpenOptionsExt, io::IntoRawHandle},
  path::{Path, PathBuf},
  sync::{Arc, LazyLock},
};

use dashmap::DashMap;
use ref_cast::RefCast;
use shared_types::HookError;
use win_api::{
  Wdk::Foundation::OBJECT_ATTRIBUTES,
  Win32::{
    Foundation::HANDLE,
    Storage::FileSystem::{
      FILE_ATTRIBUTE_OFFLINE, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAGS_AND_ATTRIBUTES,
      FILE_SHARE_DELETE, FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE,
      GETFINALPATHNAMEBYHANDLE_FLAGS, GetFinalPathNameByHandleW,
    },
  },
};

use crate::{
  extension_traits::DashExt,
  log::log_info,
  raw_ptr::UnsafeRefCast,
  virtual_paths::{MOUNT_POINT, VIRTUAL_ROOT, windows::VirtualPath},
  windows::os_types::paths::sanitise_path,
};

pub const NULL_HANDLE: HANDLE = HANDLE(std::ptr::null_mut());

// aka u32::MAX
pub const DO_NOT_HOOK: FILE_FLAGS_AND_ATTRIBUTES = FILE_ATTRIBUTE_OFFLINE;

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

pub static HANDLE_MAP: LazyLock<HandleMap> = LazyLock::new(|| Default::default());

pub fn init_handles() {
  let virtual_root = VIRTUAL_ROOT.read().unwrap().clone();
  let mut root_ancestors: Vec<_> = virtual_root.ancestors().collect();
  root_ancestors.reverse();
  for path in &root_ancestors {
    let raw_handle = std_open_dir_handle_unhooked(path).unwrap();
    HANDLE_MAP.insert(raw_handle, path);
  }

  let mount_point = MOUNT_POINT.read().unwrap().clone();
  let mut mount_ancestors: Vec<_> = mount_point
    .ancestors()
    .filter(|p| !root_ancestors.contains(p))
    .collect();
  mount_ancestors.reverse();
  for path in mount_ancestors {
    let raw_handle = std_open_dir_handle_unhooked(path).unwrap();
    HANDLE_MAP.insert(raw_handle, path);
  }
}

pub fn std_open_dir_handle_unhooked(path: impl AsRef<Path>) -> Result<Handle, HookError> {
  const DEFAULT_SHARE_MODE: u32 = FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0 | FILE_SHARE_DELETE.0;

  let handle = std::fs::File::options()
    .read(true)
    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS.0)
    // .share_mode(DEFAULT_SHARE_MODE | DO_NOT_HOOK.0)
    .attributes(DO_NOT_HOOK.0)
    .open(path.as_ref())
    .unwrap();
  Ok(handle.into_raw_handle().into())
}

#[derive(Debug, Clone)]
pub struct HandleInfo {
  pub path: PathBuf,
  pub handle: Handle,
}

#[derive(Debug, Default)]
pub struct HandleMap {
  ptr_keyed: DashMap<Handle, Arc<HandleInfo>>,
  path_keyed: DashMap<PathBuf, Arc<HandleInfo>>,
}

impl HandleMap {
  pub fn insert(&self, handle: impl Into<Handle>, path: impl AsRef<Path>) -> bool {
    let handle = handle.into();
    let path = sanitise_path(&path);
    let handle_info = Arc::new(HandleInfo {
      path: path.to_owned(),
      handle,
    });
    let inserted = self.ptr_keyed.try_insert(handle, Arc::clone(&handle_info))
      && self.path_keyed.try_insert(path.to_owned(), handle_info);

    inserted
  }

  pub fn get_by_handle(
    &self,
    handle: impl Into<Handle>,
  ) -> Option<dashmap::mapref::one::Ref<'_, Handle, Arc<HandleInfo>>> {
    self.ptr_keyed.get(&handle.into())
  }

  pub fn get_by_path<Q: Hash + Eq + ?Sized>(
    &self,
    path: &Q,
  ) -> Option<dashmap::mapref::one::Ref<'_, Handle, Arc<HandleInfo>>>
  where
    PathBuf: Borrow<Q>,
  {
    self
      .path_keyed
      .get(path)
      .and_then(|handle_ref| self.ptr_keyed.get(&handle_ref.handle))
  }

  pub unsafe fn insert_by_object_attributes(
    handle: impl Into<Handle>,
    attrs: impl ObjectAttributesExt,
  ) -> Result<bool, HookError> {
    unsafe {
      let path = attrs.path()?;
      Ok(HANDLE_MAP.insert(handle, path))
    }
  }
}

pub trait ObjectAttributesExt {
  unsafe fn path(&self) -> Result<PathBuf, HookError>;
}

impl ObjectAttributesExt for OBJECT_ATTRIBUTES {
  unsafe fn path(&self) -> Result<PathBuf, HookError> {
    unsafe {
      let stem_raw = OsString::from_wide(self.ObjectName.ref_cast()?.Buffer.as_wide());
      let stem: &Path = stem_raw.as_ref();

      if !self.RootDirectory.is_invalid() {
        let handle = Handle::ref_cast(&self.RootDirectory);
        if let Some(info) = HANDLE_MAP.get_by_handle(*handle) {
          return Ok(info.path.join(stem));
        } else if let Ok(path) = path_from_handle(handle) {
          return Ok(Path::new(&path).to_owned());
        }
      }
      Ok(stem.to_owned())
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
      handle.clone(),
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
  let path = sanitise_path(&path);
  let canon = dunce::simplified(path);

  match canon.strip_prefix(MOUNT_POINT.read()?.as_path()) {
    Ok(stem) => Ok(Some(VirtualPath {
      path: VIRTUAL_ROOT.read()?.join(stem),
      original: canon.to_path_buf(),
    })),
    _ => Ok(None),
  }
}

macro_rules! into_handle {
  () => {impl Into<crate::windows::os_types::handles::Handle>};
}

pub(crate) use into_handle;
