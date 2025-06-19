use std::path::Path;

use shared_types::HookError;
use win_api::{
  Wdk::Foundation::OBJECT_ATTRIBUTES,
  Win32::Foundation::{HANDLE, UNICODE_STRING},
};

use crate::{raw_ptr::UnsafeRefCast, windows::os_types::unicode_string::path_to_unicode_string};

pub struct ReroutedObjectAttrs {
  path_owner: widestring::U16CString,
  path: *const UNICODE_STRING,
  pub attrs: OBJECT_ATTRIBUTES,
}

impl Drop for ReroutedObjectAttrs {
  fn drop(&mut self) {
    unsafe { drop(Box::from_raw(self.path.cast_mut())) };
  }
}

pub trait RawObjectAttrsExt: UnsafeRefCast<OBJECT_ATTRIBUTES> {
  unsafe fn reroute(self, path: impl AsRef<Path>) -> Result<ReroutedObjectAttrs, HookError>;
}

impl<T: UnsafeRefCast<OBJECT_ATTRIBUTES>> RawObjectAttrsExt for T {
  unsafe fn reroute(self, path: impl AsRef<Path>) -> Result<ReroutedObjectAttrs, HookError> {
    unsafe {
      let (path_owner, unicode) = path_to_unicode_string(path)?;
      let mut reroute = ReroutedObjectAttrs {
        path_owner,
        path: Box::into_raw(Box::new(unicode)),
        attrs: self.read(),
      };

      reroute.attrs.ObjectName = reroute.path;
      reroute.attrs.RootDirectory = HANDLE(std::ptr::null_mut());

      Ok(reroute)
    }
  }
}
