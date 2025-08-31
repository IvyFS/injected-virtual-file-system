#![feature(normalize_lexically)]

use std::{error::Error, process::ExitCode};

use libtest_mimic::{Arguments, Trial, run};
use linkme::distributed_slice;

mod create_remove_directory;
mod file_edit;
mod find_first_file;
mod java;
mod nt_create;
mod nt_open;
mod nt_query_directory_file;
mod overlay;

#[distributed_slice]
pub static TESTS: [(&str, fn())];

fn main() -> Result<ExitCode, Box<dyn Error>> {
  let args = Arguments::from_args();

  Ok(run(&args, collect_tests()).exit_code())
}

fn collect_tests() -> Vec<Trial> {
  TESTS
    .into_iter()
    .map(|(name, func)| {
      Trial::test(name.strip_prefix("windows::").unwrap_or(&name), move || {
        Ok(func())
      })
    })
    .collect()
}
