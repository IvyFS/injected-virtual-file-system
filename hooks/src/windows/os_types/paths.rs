use std::path::Path;

pub const NT_PATH_PREFIX: &str = "\\??\\";

pub fn sanitise_path(path: &impl AsRef<Path>) -> &Path {
  let path = path.as_ref();
  let trimmed_front = if let Ok(trimmed) = path.strip_prefix(NT_PATH_PREFIX) {
    trimmed
  } else {
    path
  };
  if trimmed_front.ends_with("*") {
    trimmed_front.parent().unwrap()
  } else {
    trimmed_front
  }
}
