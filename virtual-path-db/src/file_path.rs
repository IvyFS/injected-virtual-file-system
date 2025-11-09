use std::{
  ffi::OsStr,
  fmt::Debug,
  ops::RangeBounds,
  path::{Path, PathBuf},
};

use rkyv::{
  Archive, Serialize,
  with::{ArchiveWith, Skip},
};

use crate::with_as_encoded_bytes::AsEncodedBytes;

#[derive(Debug, Clone, Copy, Archive, Serialize)]
#[rkyv(derive(Debug), archive_bounds(<AsEncodedBytes as ArchiveWith<&'a Path>>::Archived: Debug))]
pub struct FilePath<'a> {
  #[rkyv(with = AsEncodedBytes)]
  pub(crate) path: &'a Path,
  #[rkyv(with = Skip)]
  pub(crate) is_prefix: bool,
}

impl<'a> FilePath<'a> {
  pub fn new(path: &'a (impl AsRef<OsStr> + ?Sized)) -> Self {
    Self {
      path: Path::new(path),
      is_prefix: false,
    }
  }

  pub unsafe fn from_encoded_bytes_unchecked(bytes: &'a [u8]) -> Self {
    Self::new(Path::new(unsafe {
      OsStr::from_encoded_bytes_unchecked(bytes)
    }))
  }

  pub(crate) fn as_prefix(mut self) -> Self {
    self.is_prefix = true;
    self
  }

  /// Returns a set of bounds that includes all paths prefixed by the current
  /// path - _without_ any additional allocation.
  ///
  /// # Panics
  ///
  /// Panics if this path has a trailing slash.
  pub(crate) fn prefix_range(self) -> impl RangeBounds<FilePath<'a>> {
    // Some warning on Windows applies here as described in the `Windows`
    // section of the doc comment on `has_trailing_slash`.
    assert!(
      !self.has_trailing_slash(),
      "Cannot create a prefix range from a path with a trailing slash"
    );

    self..=self.as_prefix()
  }

  /// Returns whether this path has a trailing slash.
  ///
  /// # Windows
  ///
  /// This _might_ have false-positives if there's some character other than
  /// `/` that `OsStr::as_encoded_bytes` chooses to encode as the same bytes,
  /// or to a byte sequence ending in the same bytes, or a sequence of
  /// characters it encodes such that their encoded byte sequence ends in the
  /// same bytes.
  /// But that seems unlikely...
  fn has_trailing_slash(self) -> bool {
    #[cfg(unix)]
    fn has_trailing_slash_impl(path: &Path) -> bool {
      use std::os::unix::ffi::OsStrExt;
      path.as_os_str().as_bytes().last() == Some(&b'/')
    }
    #[cfg(windows)]
    fn has_trailing_slash_impl(path: &Path) -> bool {
      path
        .as_os_str()
        .as_encoded_bytes()
        .ends_with(OsStr::new("/").as_encoded_bytes())
    }
    has_trailing_slash_impl(self.path)
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

impl<'a> From<&'a str> for FilePath<'a> {
  fn from(value: &'a str) -> Self {
    FilePath::new(Path::new(value))
  }
}

impl<'a> AsRef<Path> for FilePath<'a> {
  fn as_ref(&self) -> &Path {
    self.path
  }
}

impl<'this, 'other> PartialEq<FilePath<'other>> for FilePath<'this> {
  fn eq(&self, other: &FilePath<'other>) -> bool {
    self.path.eq(other.path) && self.has_trailing_slash() == other.has_trailing_slash()
  }
}

impl<'other, 'this> PartialOrd<FilePath<'other>> for FilePath<'this> {
  fn partial_cmp(&self, other: &FilePath<'other>) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;

    if self.path == other.path {
      match (self.has_trailing_slash(), other.has_trailing_slash()) {
        (true, false) => return Some(Ordering::Greater),
        (false, true) => return Some(Ordering::Less),
        _ => {}
      }
    }
    match (self.is_prefix, other.is_prefix) {
      (true, false) if other.path.starts_with(self.path) => Some(Ordering::Greater),
      (false, true) if self.path.starts_with(other.path) => Some(Ordering::Less),
      _ => Some(self.path.cmp(other.path)),
    }
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
mod test {
  use std::{ffi::OsStr, ops::RangeBounds, path::Path};

  use crate::{NON_ASCII_PATH, file_path::FilePath};

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
  }

  #[test]
  fn std_path_trailing_slash_quirks() {
    // Path preserves trailing slashes
    assert_eq!("foo/", Path::new("foo/").to_str().unwrap());
    assert_eq!(OsStr::new("foo/"), Path::new("foo/").as_os_str());
    // `Path::starts_with` and `Path::eq` ignores them:
    assert_eq!(Path::new("/foo/"), Path::new("/foo"));
    assert!(Path::new("/foo").starts_with("/foo/"));
  }

  #[test]
  #[should_panic(expected = "Cannot create a prefix range from a path with a trailing slash")]
  fn range_hack_panic_on_trailing_slash() {
    let prefix = FilePath::new("foo/");
    // panics:
    _ = prefix.prefix_range();
  }

  #[test]
  fn range_containing_all_prefixes_of_path() {
    let path = FilePath::new("/foo/bar");
    let range = ..=path;

    // Rnage should contain all prefixes of path, starting from root and
    // appending subsequent components of path
    assert!(range.contains(&FilePath::new("")));
    assert!(range.contains(&FilePath::new("/")));
    assert!(range.contains(&FilePath::new("/foo")));
    assert!(range.contains(&FilePath::new("/foo/bar")));
    assert!(range.contains(&FilePath::new("/foo/bar/")));

    assert!(!range.contains(&FilePath::new("f")));
    assert!(!range.contains(&FilePath::new("/f")));
    assert!(!range.contains(&FilePath::new("/foo/b")));
  }
}
