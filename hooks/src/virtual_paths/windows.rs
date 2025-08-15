use std::{
  ffi::OsString,
  fmt::Debug,
  marker::PhantomData,
  os::windows::ffi::OsStringExt,
  path::{Path, PathBuf},
};

use ext_trait::extension;
use shared_types::HookError;
use widestring::U16CString;
use win_types::{PCSTR, PCWSTR};

use crate::{
  virtual_paths::{MOUNT_POINT, VIRTUAL_ROOT},
  windows::helpers::paths::{canonise_relative_current_dir, strip_nt_prefix},
};

#[derive(Debug)]
#[allow(dead_code)]
pub struct VirtualPath<BUF = (), RAW = ()> {
  pub path: PathBuf,
  pub original: PathBuf,
  pub raw_virtual_path: Option<BUF>,
  _raw_marker: PhantomData<RAW>,
}

impl VirtualPath {
  fn new_generic<BUF, RAW>(
    virtual_path: impl Into<PathBuf>,
    original: impl Into<PathBuf>,
  ) -> VirtualPath<BUF, RAW> {
    VirtualPath {
      path: virtual_path.into(),
      original: original.into(),
      raw_virtual_path: None,
      _raw_marker: PhantomData,
    }
  }
}

impl VirtualPath<Box<[u8]>, PCSTR> {
  pub fn as_raw_ansi(&mut self) -> PCSTR {
    let boxed = self
      .path
      .as_os_str()
      .as_encoded_bytes()
      .to_vec()
      .into_boxed_slice();
    let ansi_ptr = PCSTR::from_raw(boxed.as_ptr());
    self.raw_virtual_path = Some(boxed);

    ansi_ptr
  }
}

impl VirtualPath<U16CString, PCWSTR> {
  pub fn as_raw_wide(&mut self) -> PCWSTR {
    let owned = widestring::U16CString::from_os_str_truncate(&self.path);
    let wide_ptr = PCWSTR::from_raw(owned.as_ptr());
    self.raw_virtual_path = Some(owned);

    wide_ptr
  }
}

pub trait VirtualAsRaw<RAW> {
  fn as_raw(&mut self) -> RAW;
}

impl VirtualAsRaw<PCSTR> for VirtualPath<Box<[u8]>, PCSTR> {
  fn as_raw(&mut self) -> PCSTR {
    self.as_raw_ansi()
  }
}

impl VirtualAsRaw<PCWSTR> for VirtualPath<U16CString, PCWSTR> {
  fn as_raw(&mut self) -> PCWSTR {
    self.as_raw_wide()
  }
}

#[extension(pub trait VirtualPathOption)]
impl<BUF, RAW: Copy> Result<VirtualPath<BUF, RAW>, RAW>
where
  VirtualPath<BUF, RAW>: VirtualAsRaw<RAW>,
{
  fn as_raw_or_original(&mut self) -> RAW {
    match self {
      Ok(virt) => virt.as_raw(),
      Err(original) => *original,
    }
  }
}

pub fn get_virtual_path(path: impl AsRef<Path>) -> Result<Option<VirtualPath>, HookError> {
  get_virtual_path_or(path, ()).map(Result::ok)
}

pub type VirtualPathResult<BUF, RAW> = Result<Result<VirtualPath<BUF, RAW>, RAW>, HookError>;

pub fn get_virtual_path_or<BUF, RAW>(
  path: impl AsRef<Path>,
  alternative: RAW,
) -> VirtualPathResult<BUF, RAW> {
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
      Ok(Ok(VirtualPath::new_generic(
        rerouted_path,
        canon.to_path_buf(),
      )))
    }
    _ => Ok(Err(alternative)),
  }
}

pub fn get_virtual_path_or_ansi(path: PCSTR) -> VirtualPathResult<Box<[u8]>, PCSTR> {
  let given_path = { Path::new(unsafe { str::from_utf8_unchecked(path.as_bytes()) }) };
  let canon = canonise_relative_current_dir(given_path)?;

  get_virtual_path_or(canon, path)
}

pub fn get_virtual_path_or_wide(path: PCWSTR) -> VirtualPathResult<U16CString, PCWSTR> {
  let given_path = PathBuf::from(unsafe { OsString::from_wide(path.as_wide()) });
  let canon = canonise_relative_current_dir(given_path)?;

  get_virtual_path_or(canon, path)
}
