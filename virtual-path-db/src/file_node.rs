use std::fmt::Debug;

use rkyv::{Archive, Serialize, util::AlignedVec};

use crate::file_path::FilePath;

#[non_exhaustive]
#[derive(Debug, Clone, Archive, Serialize)]
#[rkyv(derive(Debug), archive_bounds(<FilePath<'a> as Archive>::Archived: Debug))]
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
    = ByteSliceAbstraction<'a>
  where
    Self: 'a;

  fn fixed_width() -> Option<usize> {
    None
  }

  fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
  where
    Self: 'a,
  {
    if data.is_empty() {
      Self::SelfType::Whiteout
    } else {
      Self::SelfType::RerouteUpper(FilePath::from_bytes(data))
    }
  }

  fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
  where
    Self: 'b,
  {
    match value {
      FileNode::Whiteout => ByteSliceAbstraction::Slice(&[]),
      FileNode::RerouteUpper(file_path) => {
        ByteSliceAbstraction::Slice(FilePath::as_bytes(file_path))
      }
    }
  }

  fn type_name() -> redb::TypeName {
    redb::TypeName::new("injected_virtual_file_system::FileNode")
  }
}

#[doc(hidden)]
pub enum ByteSliceAbstraction<'a> {
  Vec(Vec<u8>),
  AlignedVec(AlignedVec),
  Slice(&'a [u8]),
}

impl<'a> AsRef<[u8]> for ByteSliceAbstraction<'a> {
  fn as_ref(&self) -> &[u8] {
    match self {
      ByteSliceAbstraction::Vec(vec) => &vec,
      ByteSliceAbstraction::AlignedVec(aligned_vec) => &aligned_vec,
      ByteSliceAbstraction::Slice(slice) => slice,
    }
  }
}

#[cfg(test)]
mod test {
  use std::assert_matches::assert_matches;

  use rkyv::{access_unchecked, to_bytes};

  use crate::{
    RancorError,
    file_node::{ArchivedFileNode, FileNode},
    file_path::FilePath,
  };

  #[test]
  fn rkyv_ser_access() {
    let node = FileNode::RerouteUpper(FilePath::new("/foo/bar"));

    let bytes = to_bytes::<RancorError>(&node).unwrap();

    let archived: &ArchivedFileNode = unsafe { access_unchecked(&bytes) };

    assert_matches!(
      archived,
      ArchivedFileNode::RerouteUpper(file_path) if unsafe { FilePath::from_encoded_bytes_unchecked(&file_path.path) } == FilePath::new("/foo/bar")
    );
  }
}
