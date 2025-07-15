use std::path::PathBuf;

#[derive(Debug)]
pub struct VirtualPath {
  pub path: PathBuf,
  pub original: PathBuf,
}
