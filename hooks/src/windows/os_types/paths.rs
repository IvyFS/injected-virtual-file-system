use std::{borrow::Cow, path::Path};

use shared_types::HookError;

pub const NT_PATH_PREFIX: &str = "\\??\\";

pub fn strip_nt_prefix(path: &impl AsRef<Path>) -> &Path {
  let path = path.as_ref();
  path.strip_prefix(NT_PATH_PREFIX).unwrap_or(path)
}

pub fn canonise_relative_current_dir<'a>(given_path: impl Into<Cow<'a, Path>>) -> Result<Cow<'a, Path>, HookError> {
  let mut given_path = given_path.into();
  if given_path.is_relative() {
    let given_path = Cow::to_mut(&mut given_path);
    let mut current_dir = std::env::current_dir()?;

    // Swap the contents of these two PathBufs as we want the result to end up in the `given_path: &mut Cow`, but need
    // the current_dir to be joined/pushed onto by the contents of given_path, not the other way around
    let (out_ref, given_path) = {
      std::mem::swap(&mut current_dir, given_path);
      (given_path, current_dir)
    };
    out_ref.push(given_path);

    out_ref
      .normalize_lexically()
      .map_err(std::io::Error::other)?;
  }
  Ok(given_path)
}
