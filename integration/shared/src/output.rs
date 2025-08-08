use std::{fs::File, path::Path};

use serde::{Serialize, de::DeserializeOwned};
pub use serde_json::json;

pub fn write_output<T: Serialize + DeserializeOwned>(output: T, path: impl AsRef<Path>) {
  serde_json::to_writer_pretty(File::create(path).unwrap(), &output).unwrap()
}

pub fn read_output<T: Serialize + DeserializeOwned>(path: impl AsRef<Path>) -> T {
  serde_json::from_reader(File::open(path).unwrap()).unwrap()
}
