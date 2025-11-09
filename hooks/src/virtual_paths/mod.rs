use std::{
  any::TypeId,
  ffi::OsStr,
  ops::{Bound, RangeBounds},
  path::{Path, PathBuf},
  sync::{LazyLock, OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use redb::{ReadableDatabase, Table, TableDefinition};

#[cfg(windows)]
pub mod windows;

pub static MOUNT_POINT: LazyLock<RwLock<PathBuf>> = LazyLock::new(|| RwLock::new(PathBuf::new()));
pub static VIRTUAL_ROOT: LazyLock<RwLock<PathBuf>> = LazyLock::new(|| RwLock::new(PathBuf::new()));

static VIRTUAL_PATHS: OnceLock<redb::Database> = OnceLock::new();

const TABLE: TableDefinition<FilePath, FileNode> =
  TableDefinition::new("injected_virtual_file_system::TABLE::VIRTUAL_PATHS");

#[derive(Debug, thiserror::Error)]
pub enum FileSystemError {
  #[error("reroute db transaction error {0}")]
  TransactionError(#[from] redb::TransactionError),
  #[error("reroute db table error {0}")]
  TableError(#[from] redb::TableError),
  #[error("reroute db storage error {0}")]
  StorageError(#[from] redb::StorageError),
}

#[derive(Clone, Copy, Default)]
pub struct FileSystem;

type FileSystemTable<'a> = Table<'a, FilePath<'static>, FileNode<'static>>;

impl FileSystem {
  pub fn init() -> Self {
    VIRTUAL_PATHS
      .set(
        redb::Builder::new()
          .create_with_backend(redb::backends::InMemoryBackend::new())
          .unwrap(),
      )
      .unwrap();

    Self
  }

  fn write(self) -> Result<redb::WriteTransaction, redb::TransactionError> {
    unsafe { get_db() }.begin_write()
  }

  fn read(self) -> redb::ReadTransaction {
    unsafe { get_db() }.begin_read().unwrap()
  }

  fn table_write<'txn, T>(
    self,
    writer: impl FnOnce(&mut FileSystemTable) -> Result<T, FileSystemError>,
  ) -> Result<T, FileSystemError> {
    let write_tx = self.write()?;
    let mut write_table = write_tx.open_table(TABLE)?;
    writer(&mut write_table)
  }

  pub fn add_redirect<'a>(
    self,
    lower_path: impl Into<FilePath<'a>>,
    upper_path: impl AsRef<Path>,
  ) -> Result<bool, FileSystemError> {
    self.table_write(|table| {
      let key = lower_path.into();

      // Remove any existing reroutes under this prefix
      table.retain_in::<FilePath<'_>, _>(key.prefix_range(), |_, _| false)?;

      Ok(table.insert(key, FileNode::RerouteUpper(FilePath::new(upper_path.as_ref())))?.is_some())
    })
  }

  pub fn add_whiteout<'a>(lower_path: impl Into<FilePath<'a>>) {
    // let mut trie = Self::write();
    // let lower_path = lower_path.into();

    // while let Some(mut descendant) = trie.subtrie_mut(&lower_path) {
    //   descendant.remove(&lower_path);
    // }

    // trie.map_with_default(
    //   lower_path,
    //   |node| {
    //     node.reroute = Reroute::Whiteout;
    //     node.version += 1;
    //   },
    //   FileNode {
    //     reroute: Reroute::Whiteout,
    //     version: 0,
    //   },
    // );
    // trie.subtrie_mut()
  }

  pub fn remove_path<'a>(lower_path: impl Into<FilePath<'a>>) {
    // Self::write().remove(&lower_path.into());
  }

  // fn remove_descendants(trie: &mut FSTrie, key: &FilePath) -> bool {
  //   let mut removed = false;

  //   if let Some(raw_desc_key) = trie
  //     .get_raw_descendant(key)
  //     .as_ref()
  //     .and_then(TrieCommon::key)
  //     .cloned()
  //   {
  //     if let Some(mut subtrie) = trie.subtrie_mut(&raw_desc_key) {
  //       // subtrie.iter().next().unwrap().
  //     }
  //   }

  //   if let Some(subtrie) = trie.subtrie_mut(key) {
  //     let subkeys: Vec<_> = (&subtrie).keys().collect();
  //     // for subkey in subkeys {
  //     //   if trie.remove(&subkey).is_some() {
  //     //     removed = true
  //     //   }
  //     // }
  //     panic!("{subkeys:?}")
  //   }

  //   removed
  // }
}

unsafe fn get_db<'a>() -> &'a redb::Database {
  VIRTUAL_PATHS.get().unwrap()
}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum FileNode<'a> {
  Whiteout,
  RerouteUpper(FilePath<'a>),
}

impl<'f> redb::Value for FileNode<'f> {
  type SelfType<'a>
    = FileNode<'a>
  where
    Self: 'a;

  type AsBytes<'a>
    = &'a [u8]
  where
    Self: 'a;

  fn fixed_width() -> Option<usize> {
    None
  }

  fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
  where
    Self: 'a,
  {
    match data.split_first() {
      None => Self::SelfType::Whiteout,
      Some((1, rem)) => Self::SelfType::RerouteUpper(FilePath::from_bytes(rem)),
      Some((disc, _)) => panic!("Unknown enum variant discriminator {disc}"),
    }
  }

  fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
  where
    Self: 'b,
  {
    match value {
      FileNode::Whiteout => &[],
      FileNode::RerouteUpper(file_path) => FilePath::as_bytes(file_path),
    }
  }

  fn type_name() -> redb::TypeName {
    redb::TypeName::new("injected_virtual_file_system::FileNode")
  }
}

#[derive(Debug, Eq, Clone, Copy)]
pub struct FilePath<'a> {
  path: &'a Path,
  is_prefix: bool,
}

impl<'a> FilePath<'a> {
  pub fn new(path: &'a (impl AsRef<OsStr> + ?Sized)) -> Self {
    Self {
      path: Path::new(path),
      is_prefix: false,
    }
  }

  fn as_prefix(mut self) -> Self {
    self.is_prefix = true;
    self
  }

  /// Returns a set of bounds that includes all paths prefixed by the current
  /// path - _without_ any additional allocation.
  ///
  /// # Panics
  ///
  /// Panics if this path has a trailing slash.
  fn prefix_range(self) -> impl RangeBounds<FilePath<'a>> {
    // This _might_ have false-positives if there's some character other than
    // '/' that `OsStr::as_encoded_bytes` chooses to encode as the same bytes,
    // or to a byte sequence ending in the same bytes, or a sequence of
    // characters it encodes such that their encoded byte sequence ends in the
    // same bytes.
    // But that seems unlikely...
    assert!(
      !self
        .path
        .as_os_str()
        .as_encoded_bytes()
        .ends_with(OsStr::new("/").as_encoded_bytes()),
      "Cannot create a prefix range from a path with a trailing slash"
    );

    self..=self.as_prefix()
  }
}

impl<'a> From<&'a Path> for FilePath<'a> {
  fn from(value: &'a Path) -> Self {
    Self::new(value)
  }
}

impl<'a> From<&'a PathBuf> for FilePath<'a> {
  fn from(value: &'a PathBuf) -> Self {
    Self::new(value)
  }
}

impl<'a> AsRef<Path> for FilePath<'a> {
  fn as_ref(&self) -> &Path {
    self.path
  }
}

impl<'this, 'other> PartialEq<FilePath<'other>> for FilePath<'this> {
  fn eq(&self, other: &FilePath<'other>) -> bool {
    self.path.eq(other.path)
  }
}

impl<'other, 'this> PartialOrd<FilePath<'other>> for FilePath<'this> {
  fn partial_cmp(&self, other: &FilePath<'other>) -> Option<std::cmp::Ordering> {
    if self != other {
      match (self.is_prefix, other.is_prefix) {
        (true, false) => {
          if other.path.starts_with(self.path) {
            return Some(std::cmp::Ordering::Greater);
          }
        }
        (false, true) => {
          if self.path.starts_with(other.path) {
            return Some(std::cmp::Ordering::Less);
          }
        }
        _ => {}
      }
    }
    Some(self.path.cmp(other.path))
  }
}

impl<'f> redb::Value for FilePath<'f> {
  type SelfType<'a>
    = FilePath<'a>
  where
    Self: 'a;

  type AsBytes<'a>
    = &'a [u8]
  where
    Self: 'a;

  fn fixed_width() -> Option<usize> {
    None
  }

  fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
  where
    Self: 'a,
  {
    Self::SelfType::new(Path::new(unsafe {
      OsStr::from_encoded_bytes_unchecked(data)
    }))
  }

  fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
  where
    Self: 'b,
  {
    value.path.as_os_str().as_encoded_bytes()
  }

  fn type_name() -> redb::TypeName {
    redb::TypeName::new("injected_virtual_file_system::FilePath")
  }
}

impl<'f> redb::Key for FilePath<'f> {
  fn compare(data1: &[u8], data2: &[u8]) -> std::cmp::Ordering {
    data1.cmp(data2)
  }
}

#[cfg(test)]
mod tests {
  use std::ffi::OsString;

  use super::*;

  #[test]
  fn basic_file_path_encoding_round_trip() {
    let path = "foo/bar/baz";

    let encoded = FilePath::new(Path::new(path));
    let decoded: &Path = encoded.as_ref();

    assert_eq!(path, decoded.to_string_lossy())
  }

  #[test]
  fn non_alpha_file_path_encoding_round_trip() {
    let path = "foo/bar\\#$%_++'[]``.";

    let encoded = FilePath::new(Path::new(path));
    let decoded: &Path = encoded.as_ref();

    assert_eq!(path, decoded.to_string_lossy())
  }

  #[test]
  fn trailing_slash_file_path_encoding_round_trip() {
    let path = "foo/bar/baz/";

    let encoded = FilePath::new(Path::new(path));
    let decoded: &Path = encoded.as_ref();

    assert_eq!(path, decoded.to_string_lossy())
  }

  const NON_ASCII_PATH: &'static str = "😂/𤭢";

  #[cfg(windows)]
  #[test]
  fn unpaired_surrogate_file_path_encoding_round_trip() {
    use redb::Value;
    use std::os::windows::ffi::OsStringExt;

    // Non-ASCII characters are encoded with two u16s aka as "surrogate pairs"
    let utf_16_path: Vec<u16> = NON_ASCII_PATH.encode_utf16().collect();
    // Create a WTF-8 string by slicing into the UTF-16 array so we omit the
    // first half of the first non-ASCII character's surrogate pair and the
    // second half of the second non-ASCII character's surrogate pair.
    let wtf_8_path = OsString::from_wide(&utf_16_path[1..utf_16_path.len() - 1]);
    let file_path = FilePath::new(&wtf_8_path);
    let file_path_bytes = FilePath::as_bytes(&file_path);
    let decoded_file_path = FilePath::from_bytes(file_path_bytes);
    let file_path_as_path: &Path = decoded_file_path.as_ref();

    assert_eq!("\"\\u{de02}/\\u{d852}\"", format!("{file_path_as_path:?}"));
  }

  #[test]
  fn naive_prefix_range_fails() {
    let range = "foo"..="foo";

    assert!(!range.contains(&"foobar"))
  }

  #[test]
  fn range_hack_using_is_prefix_field_and_custom_ord_impl() {
    let range = FilePath::new("foo").prefix_range();

    // sub-dir
    assert!(range.contains(&FilePath::new("foo/bar")));
    // sub-dir, double checking type conversions
    assert!(range.contains(&FilePath::new(Path::new("foo/bar"))));
    // sub-sub-dir
    assert!(range.contains(&FilePath::new("foo/bar/baz")));
    // sub-dir starts with prev char to last in prefix (n -> o)
    assert!(range.contains(&FilePath::new("foo/n")));
    // sub-dir starts with next char to last in prefix (o -> p)
    assert!(range.contains(&FilePath::new("foo/n")));
    // sub-dir starts with non-alpha
    assert!(range.contains(&FilePath::new("foo/0")));
    // sub-dir starts with non-alphanumeric
    assert!(range.contains(&FilePath::new("foo/!")));
    // sub-dir starts with dot
    assert!(range.contains(&FilePath::new("foo/.")));
    // sub-dir is ".." (technically incorrect, but implementation should pass)
    assert!(range.contains(&FilePath::new("foo/..")));
    // sub-dir contains non-ASCII chars
    assert!(range.contains(&FilePath::new(&format!("foo/{NON_ASCII_PATH}"))));
    // not a sub-dir
    assert!(!range.contains(&FilePath::new(Path::new("fop/bar"))));
    // not a sub-dir
    assert!(!range.contains(&FilePath::new(Path::new("fon/bar"))));
    // not a sub-dir, however ignoring separators is prefixed by prefix
    assert!(!range.contains(&FilePath::new(Path::new("foob/ar"))));
  }

  #[test]
  fn range_hack_prefix_from_root() {
    let range = FilePath::new("/foo/bar").prefix_range();

    // sub-dir
    assert!(range.contains(&FilePath::new("/foo/bar/baz")));
    // sub-dir is ".." (technically incorrect, but implementation should pass)
    assert!(range.contains(&FilePath::new("/foo/bar/baz/..")));
    // not a sub-dir
    assert!(!range.contains(&FilePath::new(Path::new("/fop/bar"))));
    // not a sub-dir
    assert!(!range.contains(&FilePath::new(Path::new("/fon/bar"))));
    // not a sub-dir, however ignoring separators is prefixed by prefix
    assert!(!range.contains(&FilePath::new(Path::new("/foob/ar"))));
  }

  #[test]
  #[ignore = "`FilePath::prefix_range` panics when called on a path with a trailing slash.
Otherwise the assertions in this test should hold."]
  fn range_hack_handles_prefix_with_trailing_slash() {
    let range = FilePath::new("foo/").prefix_range();

    assert!(range.contains(&FilePath::new("foo/bar")));
    assert!(!range.contains(&FilePath::new("fop/bar")));
    assert!(!range.contains(&FilePath::new("foob/ar")));

    // Main point of potential confusion:
    assert!(range.contains(&FilePath::new("foo")));
    assert!(range.contains(&FilePath::new("foo/")));
    assert!(range.contains(&FilePath::new("foo//")));

    // The above passes because whilst `Path` preserves trailing slashes:
    assert_eq!("foo/", Path::new("foo/").to_str().unwrap());
    assert_eq!(OsStr::new("foo/"), Path::new("foo/").as_os_str());
    // `Path::starts_with` and `Path::eq` ignores them:
    assert_eq!(Path::new("foo/"), Path::new("foo"));
  }

  #[test]
  #[should_panic(expected = "Cannot create a prefix range from a path with a trailing slash")]
  fn range_hack_panic_on_trailing_slash() {
    let prefix = FilePath::new("foo/");
    // panics:
    _ = prefix.prefix_range();
  }
}
