use std::cell::UnsafeCell;

#[repr(transparent)]
pub struct UnsafeSyncCell<T: Sync>(UnsafeCell<T>);

impl<T: Sync> UnsafeSyncCell<T> {
  pub const fn new(value: T) -> Self {
    Self(UnsafeCell::new(value))
  }

  pub const fn get(&self) -> *mut T {
    self.0.get()
  }

  pub const unsafe fn as_mut_unchecked(&self) -> &mut T {
    unsafe { &mut *self.get() }
  }
}

unsafe impl<T: Sync> Sync for UnsafeSyncCell<T> {}
