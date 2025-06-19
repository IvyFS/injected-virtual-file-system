use core::any::type_name;

use shared_types::HookError;

pub trait UnsafeRefCast<T> {
  unsafe fn read(self) -> T;

  unsafe fn ref_cast<'a>(self) -> Result<&'a T, HookError>;

  unsafe fn mut_cast<'a>(self) -> Result<&'a mut T, HookError>;
}

impl<T> UnsafeRefCast<T> for *const T {
  unsafe fn read(self) -> T {
    unsafe { self.read() }
  }

  unsafe fn ref_cast<'a>(self) -> Result<&'a T, HookError> {
    unsafe {
      self.as_ref().ok_or_else(|| HookError::RawConstPtrCast {
        typ: type_name::<T>().to_owned(),
      })
    }
  }

  unsafe fn mut_cast<'a>(self) -> Result<&'a mut T, HookError> {
    unsafe {
      self
        .cast_mut()
        .as_mut()
        .ok_or_else(|| HookError::RawConstPtrCast {
          typ: type_name::<T>().to_owned(),
        })
    }
  }
}

impl<T> UnsafeRefCast<T> for *mut T {
  unsafe fn read(self) -> T {
    unsafe { self.read() }
  }

  unsafe fn ref_cast<'a>(self) -> Result<&'a T, HookError> {
    unsafe {
      self.as_ref().ok_or_else(|| HookError::RawConstPtrCast {
        typ: type_name::<T>().to_owned(),
      })
    }
  }

  unsafe fn mut_cast<'a>(self) -> Result<&'a mut T, HookError> {
    unsafe {
      self.as_mut().ok_or_else(|| HookError::RawConstPtrCast {
        typ: type_name::<T>().to_owned(),
      })
    }
  }
}
