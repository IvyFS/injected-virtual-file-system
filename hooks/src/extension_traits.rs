use core::hash::Hash;

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
