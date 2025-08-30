use std::cell::UnsafeCell;

#[repr(transparent)]
pub struct SyncUnsafeCell<T: Sync>(UnsafeCell<T>);

impl<T: Sync> SyncUnsafeCell<T> {
  pub const fn new(value: T) -> Self {
    Self(UnsafeCell::new(value))
  }

  pub const fn get(&self) -> *mut T {
    self.0.get()
  }
}

unsafe impl<T: Sync> Sync for SyncUnsafeCell<T> {}

#[repr(transparent)]
pub struct SendPtr(pub *mut std::ffi::c_void);

unsafe impl Send for SendPtr {}
