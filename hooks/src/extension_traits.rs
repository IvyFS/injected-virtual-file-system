use core::hash::Hash;
use std::ops::ControlFlow;

use dashmap::DashMap;
use ext_trait::extension;

#[extension(pub trait DashExt)]
impl<K: Eq + Hash, V> DashMap<K, V> {
  fn try_insert(&self, key: K, value: V) -> bool {
    let mut inserted = false;
    self.entry(key).or_insert_with(|| {
      inserted = true;
      value
    });

    inserted
  }

  fn get_or_insert_with(
    &self,
    key: K,
    with: impl FnOnce() -> V,
  ) -> dashmap::mapref::one::RefMut<'_, K, V> {
    self.entry(key).or_insert_with(with)
  }

  fn get_or_try_insert_with<E>(
    &self,
    key: K,
    try_with: impl FnOnce() -> Result<V, E>,
  ) -> Result<dashmap::mapref::one::RefMut<'_, K, V>, E> {
    self.entry(key).or_try_insert_with(try_with)
  }
}

#[extension(pub trait ControlContinues)]
impl<T: Sized, B: Sized> T {
  fn continues(self) -> ControlFlow<B, T> {
    ControlFlow::Continue(self)
  }
}

#[extension(pub trait ControlBreaks)]
impl<T: Sized, C: Sized> T {
  fn breaks(self) -> ControlFlow<T, C> {
    ControlFlow::Break(self)
  }
}

#[extension(pub trait ResultIntoControlFlow)]
impl<T, E> Result<T, E>
where
  Self: Sized,
{
  fn err_continues(self) -> Result<T, ControlFlow<E, E>> {
    self.map_err(ControlFlow::Continue)
  }

  fn err_breaks(self) -> Result<T, ControlFlow<E, E>> {
    self.map_err(ControlFlow::Break)
  }

  fn map_continues(self) -> Result<ControlFlow<T, T>, E> {
    self.map(ControlFlow::Continue)
  }

  fn map_breaks(self) -> Result<ControlFlow<T, T>, E> {
    self.map(ControlFlow::Break)
  }
}

#[extension(pub trait ControlFlowExt)]
impl<T: Sized> ControlFlow<T, T> {
  fn map_either<U>(self, map: impl FnOnce(T) -> U) -> ControlFlow<U, U> {
    match self {
      ControlFlow::Continue(val) => ControlFlow::Continue(map(val)),
      ControlFlow::Break(val) => ControlFlow::Break(map(val)),
    }
  }
}
