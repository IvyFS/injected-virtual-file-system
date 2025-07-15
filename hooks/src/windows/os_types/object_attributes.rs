use std::path::Path;

use shared_types::HookError;
use win_api::{Wdk::Foundation::OBJECT_ATTRIBUTES, Win32::Foundation::HANDLE};

use crate::{raw_ptr::UnsafeRefCast, windows::os_types::unicode_string::OwnedUnicodeString};

#[derive(Debug)]
pub struct ReroutedObjectAttrs {
  pub unicode_path: OwnedUnicodeString,
  pub attrs: OBJECT_ATTRIBUTES,
}

pub trait RawObjectAttrsExt: UnsafeRefCast<OBJECT_ATTRIBUTES> {
  unsafe fn reroute(self, path: impl AsRef<Path>) -> Result<ReroutedObjectAttrs, HookError>;
}

impl<T: UnsafeRefCast<OBJECT_ATTRIBUTES>> RawObjectAttrsExt for T {
  unsafe fn reroute(self, path: impl AsRef<Path>) -> Result<ReroutedObjectAttrs, HookError> {
    unsafe {
      let unicode_path = OwnedUnicodeString::path_to_unicode_nt_path(path)?;
      let mut reroute = ReroutedObjectAttrs {
        unicode_path,
        attrs: self.read(),
      };

      reroute.attrs.ObjectName = reroute.unicode_path.unicode_ptr;
      reroute.attrs.RootDirectory = HANDLE(std::ptr::null_mut());

      Ok(reroute)
    }
  }
}
