use core::any::type_name;

use shared_types::HookError;

#[allow(dead_code)]
pub trait UnsafePtrCast<T> {
  unsafe fn read(self) -> T;

  unsafe fn ref_cast<'a>(self) -> Result<&'a T, UnsafePtrCastError>;

  unsafe fn mut_cast<'a>(self) -> Result<&'a mut T, UnsafePtrCastError>;
}

impl<T> UnsafePtrCast<T> for *const T {
  unsafe fn read(self) -> T {
    unsafe { self.read() }
  }

  unsafe fn ref_cast<'a>(self) -> Result<&'a T, UnsafePtrCastError> {
    unsafe {
      self.as_ref().ok_or_else(|| UnsafePtrCastError {
        typ: type_name::<T>().to_owned(),
        from: RawPtrType::Const,
        to: ReferenceType::Immutable,
      })
    }
  }

  unsafe fn mut_cast<'a>(self) -> Result<&'a mut T, UnsafePtrCastError> {
    unsafe {
      self.cast_mut().as_mut().ok_or_else(|| UnsafePtrCastError {
        typ: type_name::<T>().to_owned(),
        from: RawPtrType::Const,
        to: ReferenceType::Mutable,
      })
    }
  }
}

impl<T> UnsafePtrCast<T> for *mut T {
  unsafe fn read(self) -> T {
    unsafe { self.read() }
  }

  unsafe fn ref_cast<'a>(self) -> Result<&'a T, UnsafePtrCastError> {
    unsafe {
      self.as_ref().ok_or_else(|| UnsafePtrCastError {
        typ: type_name::<T>().to_owned(),
        from: RawPtrType::Mut,
        to: ReferenceType::Immutable,
      })
    }
  }

  unsafe fn mut_cast<'a>(self) -> Result<&'a mut T, UnsafePtrCastError> {
    unsafe {
      self.as_mut().ok_or_else(|| UnsafePtrCastError {
        typ: type_name::<T>().to_owned(),
        from: RawPtrType::Mut,
        to: ReferenceType::Mutable,
      })
    }
  }
}

#[derive(Debug, Clone)]
pub struct UnsafePtrCastError {
  typ: String,
  from: RawPtrType,
  to: ReferenceType,
}

#[derive(Debug, Clone, Copy)]
pub enum RawPtrType {
  Const,
  Mut,
}

#[derive(Debug, Clone, Copy)]
pub enum ReferenceType {
  Immutable,
  Mutable,
}

impl From<UnsafePtrCastError> for HookError {
  fn from(val: UnsafePtrCastError) -> Self {
    let mutable_ref = matches!(val.to, ReferenceType::Mutable);
    match val.from {
      RawPtrType::Const => HookError::RawConstPtrCast {
        typ: val.typ,
        mutable_ref,
      },
      RawPtrType::Mut => HookError::RawMutPtrCast {
        typ: val.typ,
        mutable_ref,
      },
    }
  }
}
