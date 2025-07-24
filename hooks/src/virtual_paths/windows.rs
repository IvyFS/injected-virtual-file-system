use std::path::PathBuf;

#[derive(Debug)]
#[allow(dead_code)]
pub struct VirtualPath {
  pub path: PathBuf,
  pub original: PathBuf,
}
