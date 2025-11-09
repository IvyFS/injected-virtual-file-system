#![feature(assert_matches)]

use std::{
  path::{Path, PathBuf},
  sync::{LazyLock, OnceLock, RwLock},
};

use redb::{ReadableDatabase, Table, TableDefinition};

use crate::{file_node::FileNode, file_path::FilePath};

mod file_node;
mod file_path;
mod with_as_encoded_bytes;

type RancorError = rkyv::rancor::Error;

const TABLE: TableDefinition<FilePath, file_node::FileNode> =
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

#[derive(Clone, Copy)]
pub struct FileSystem(&'static OnceLock<redb::Database>);

type FileSystemTable<'a> = Table<'a, file_path::FilePath<'static>, file_node::FileNode<'static>>;

impl FileSystem {
  pub fn init(db_cell: &'static OnceLock<redb::Database>) -> Self {
    db_cell
      .set(
        redb::Builder::new()
          .create_with_backend(redb::backends::InMemoryBackend::new())
          .unwrap(),
      )
      .unwrap();

    Self(db_cell)
  }

  fn db<'a>(self) -> &'a redb::Database {
    self.0.get().unwrap()
  }

  fn write(self) -> Result<redb::WriteTransaction, redb::TransactionError> {
    self.db().begin_write()
  }

  fn read(self) -> redb::ReadTransaction {
    self.db().begin_read().unwrap()
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
      remove_prefix(table, key)?;

      Ok(
        table
          .insert(
            key,
            FileNode::RerouteUpper(FilePath::new(upper_path.as_ref())),
          )?
          .is_some(),
      )
    })
  }

  pub fn add_whiteout<'a>(
    self,
    lower_path: impl Into<FilePath<'a>>,
  ) -> Result<bool, FileSystemError> {
    self.table_write(|table| {
      let key = lower_path.into();

      remove_prefix(table, key)?;

      Ok(table.insert(key, FileNode::Whiteout)?.is_some())
    })
  }
}

fn remove_prefix<'t, 'a>(
  table: &'t mut FileSystemTable,
  prefix: FilePath<'a>,
) -> Result<(), FileSystemError> {
  Ok(table.retain_in::<FilePath<'_>, _>(prefix.prefix_range(), |_, _| false)?)
}

#[cfg(test)]
const NON_ASCII_PATH: &'static str = "😂/𤭢";

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
}
