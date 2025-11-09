use std::path::Path;

use rkyv::{
  rancor::Fallible,
  ser::{Allocator, Writer},
  vec::{ArchivedVec, VecResolver},
  with::{ArchiveWith, SerializeWith},
};

pub(crate) struct AsEncodedBytes;

impl ArchiveWith<&Path> for AsEncodedBytes {
  type Archived = ArchivedVec<u8>;
  type Resolver = VecResolver;

  fn resolve_with(field: &&Path, resolver: Self::Resolver, out: rkyv::Place<Self::Archived>) {
    ArchivedVec::resolve_from_slice(field.as_os_str().as_encoded_bytes(), resolver, out);
  }
}

impl<S: Fallible + Allocator + Writer + ?Sized> SerializeWith<&Path, S> for AsEncodedBytes {
  fn serialize_with(
    field: &&Path,
    serializer: &mut S,
  ) -> Result<Self::Resolver, <S as Fallible>::Error> {
    ArchivedVec::serialize_from_slice(field.as_os_str().as_encoded_bytes(), serializer)
  }
}
