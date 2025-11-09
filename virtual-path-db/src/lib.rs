#![feature(assert_matches)]

use std::{
  borrow::Cow,
  ops::ControlFlow,
  path::{Path, PathBuf},
  sync::{LazyLock, OnceLock, RwLock},
};

use redb::{
  AccessGuard, ReadOnlyTable, ReadableDatabase, ReadableTable, Result, Table, TableDefinition,
};

use crate::{file_node::FileNode, file_path::FilePath};

mod file_node;
mod file_path;
mod with_as_encoded_bytes;

type RancorError = rkyv::rancor::Error;

const TABLE: TableDefinition<FilePath, file_node::FileNode> =
  TableDefinition::new("injected_virtual_file_system::TABLE::VIRTUAL_PATHS");

#[derive(Debug, thiserror::Error)]
pub enum VirtualPathError {
  #[error("virtual-path-db transaction error {0}")]
  TransactionError(#[from] redb::TransactionError),
  #[error("virtual-path-db table error {0}")]
  TableError(#[from] redb::TableError),
  #[error("virtual-path-db storage error {0}")]
  StorageError(#[from] redb::StorageError),
  #[error("virtual-path-db commit error {0}")]
  CommitError(#[from] redb::CommitError),
}

#[derive(Clone, Copy)]
pub struct VirtualPathDatabase(&'static OnceLock<redb::Database>);

type ReadWritePathTable<'a> = Table<'a, file_path::FilePath<'static>, file_node::FileNode<'static>>;
type ReadOnlyPathTable = ReadOnlyTable<file_path::FilePath<'static>, file_node::FileNode<'static>>;

impl VirtualPathDatabase {
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

  fn read(self) -> Result<redb::ReadTransaction, redb::TransactionError> {
    self.db().begin_read()
  }

  fn table_write<T>(
    self,
    writer: impl FnOnce(ReadWritePathTable) -> Result<T, VirtualPathError>,
  ) -> Result<T, VirtualPathError> {
    let write_tx = self.write()?;
    let write_table = write_tx.open_table(TABLE)?;
    let res = writer(write_table)?;
    write_tx.commit()?;
    Ok(res)
  }

  fn table_read<T>(
    self,
    reader: impl FnOnce(&ReadOnlyPathTable) -> Result<T, VirtualPathError>,
  ) -> Result<T, VirtualPathError> {
    let read_tx = self.read()?;
    let read_table = read_tx.open_table(TABLE)?;
    reader(&read_table)
  }

  pub fn add_redirect<'a>(
    self,
    lower_path: impl Into<FilePath<'a>>,
    upper_path: impl AsRef<Path>,
  ) -> Result<bool, VirtualPathError> {
    self.table_write(|mut table| {
      let key = lower_path.into();

      // Remove any existing reroutes under this prefix
      remove_prefix(&mut table, key)?;

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
  ) -> Result<bool, VirtualPathError> {
    self.table_write(|mut table| {
      let key = lower_path.into();

      remove_prefix(&mut table, key)?;

      Ok(table.insert(key, FileNode::Whiteout)?.is_some())
    })
  }

  pub fn resolve<'a>(
    self,
    lower_path: impl Into<FilePath<'a>>,
  ) -> Result<Option<Cow<'a, Path>>, VirtualPathError> {
    self.table_read(|table| {
      let key = lower_path.into();

      if let Some((found_key, found_node)) = get_furthest_ancestor(table, &key.path)? {
        let found_node = found_node.value();
        match found_node {
          FileNode::Whiteout => Ok(None),
          FileNode::RerouteUpper(upper_prefix) => {
            let suffix = key
              .path
              .strip_prefix(found_key)
              .expect("key must always be a prefix");
            if Path::new("") == suffix {
              Ok(Some(upper_prefix.path.to_owned().into()))
            } else {
              Ok(Some(upper_prefix.path.join(suffix).into()))
            }
          }
        }
      } else {
        Ok(Some(key.path.into()))
      }
    })
  }
}

fn remove_prefix<'t, 'a>(
  table: &'t mut ReadWritePathTable,
  prefix: FilePath<'a>,
) -> Result<(), VirtualPathError> {
  Ok(table.retain_in::<FilePath<'_>, _>(prefix.prefix_range(), |_, _| false)?)
}

/// Gets the key-value pair in the table whose key is the _shortest_ prefix of the given `Path`.
/// Returns `None` if no such pair exists.
fn get_furthest_ancestor<'t, 'p>(
  table: &'t impl ReadableTable<FilePath<'static>, FileNode<'static>>,
  path: &'p impl AsRef<Path>,
) -> Result<Option<(&'p Path, AccessGuard<'t, FileNode<'static>>)>, VirtualPathError> {
  let stack: Vec<_> = path.as_ref().ancestors().collect();
  for parent in stack.into_iter().rev() {
    if let Some(node) = table.get(FilePath::new(parent))? {
      return Ok(Some((parent, node)));
    }
  }

  Ok(None)
}

/// Gets the key-value pair in the table whose key is the _longest_ prefix of the given `Path`.
/// Returns `None` if no such pair exists.
fn get_nearest_ancestor<'t, 'p>(
  table: &'t impl ReadableTable<FilePath<'static>, FileNode<'static>>,
  path: &'p impl AsRef<Path>,
) -> Result<Option<(&'p Path, AccessGuard<'t, FileNode<'static>>)>, VirtualPathError> {
  for path in path.as_ref().ancestors() {
    if let Some(node) = table.get(FilePath::new(path))? {
      return Ok(Some((path, node)));
    }
  }

  Ok(None)
}

#[cfg(test)]
const NON_ASCII_PATH: &'static str = "😂/𤭢";

#[cfg(test)]
mod tests {
  use std::{assert_matches::assert_matches, borrow::Cow, path::Path, sync::OnceLock};

  use crate::VirtualPathDatabase;

  fn init_test_db() -> VirtualPathDatabase {
    static DB_CELL: OnceLock<redb::Database> = OnceLock::new();
    VirtualPathDatabase::init(&DB_CELL)
  }

  #[test]
  fn virtual_path_db_basic() {
    let db = init_test_db();

    let never_redirect = Path::new("/never/redirect");
    let always_redirect = Path::new("/always/redirect");
    let redirect_dest = Path::new("/redirect");
    let whiteout = Path::new("/doesnt/exist");
    db.add_redirect(always_redirect, redirect_dest).unwrap();
    db.add_whiteout(whiteout).unwrap();

    // Redirects resolve correctly
    assert_eq!(redirect_dest, db.resolve(always_redirect).unwrap().unwrap());
    assert_eq!(
      redirect_dest.join("foo"),
      db.resolve(&always_redirect.join("foo")).unwrap().unwrap()
    );
    assert_eq!(
      redirect_dest.join("foo/bar"),
      db.resolve(&always_redirect.join("foo/bar"))
        .unwrap()
        .unwrap()
    );

    // Non-redirect paths resolve correctly - output always == input
    assert_eq!(never_redirect, db.resolve(never_redirect).unwrap().unwrap());
    let never_redirect_child = never_redirect.join("foo");
    let never_redirect_child_resolved = db.resolve(&never_redirect_child).unwrap().unwrap();
    assert_eq!(never_redirect_child, never_redirect_child_resolved);
    assert_matches!(never_redirect_child_resolved, Cow::Borrowed(_));
    assert_eq!(Path::new("/al"), db.resolve("/al").unwrap().unwrap());
    assert_eq!(
      Path::new("/always"),
      db.resolve("/always").unwrap().unwrap()
    );
    assert_eq!(
      Path::new("/always/re"),
      db.resolve("/always/re").unwrap().unwrap()
    );
    assert_eq!(
      Path::new("/always/redirectnot"),
      db.resolve("/always/redirectnot").unwrap().unwrap()
    );

    // Whiteouts return None
    assert_eq!(None, db.resolve(whiteout).unwrap());
    assert_eq!(None, db.resolve(&whiteout.join("foo")).unwrap());
    assert_eq!(None, db.resolve(&whiteout.join("foo/bar")).unwrap());
  }
}
