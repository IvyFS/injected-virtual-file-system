use std::path::{Component, Path};

pub const NT_PATH_PREFIX: &str = "\\??\\";

pub fn sanitise_path(path: &impl AsRef<Path>) -> (&Path, bool) {
  let path = path.as_ref();
  let trimmed_front = if let Ok(trimmed) = path.strip_prefix(NT_PATH_PREFIX) {
    trimmed
  } else {
    path
  };
  if trimmed_front.ends_with("*") {
    (trimmed_front.parent().unwrap(), true)
  } else {
    (trimmed_front, false)
  }
}

/// Checks if path fragment is relative
pub fn fragment_is_relative(path: impl AsRef<Path>) -> bool {
  path
    .as_ref()
    .components()
    .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}
