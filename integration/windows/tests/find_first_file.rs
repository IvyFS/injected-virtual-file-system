use std::path::Path;

use proc_macros::ctest;

use integration_shared::{
  TestHarness,
  output::{json, read_output, write_output},
};

const FIND_FIRST_FILE: &str = env!("CARGO_BIN_EXE_FIND_FIRST_FILE");

#[ctest(crate::TESTS)]
fn absolute_redirect() {
  let mut test_harness = TestHarness::new(FIND_FIRST_FILE);
  test_harness.set_args([
    test_harness
      .mount_dir
      .path()
      .join("*")
      .display()
      .to_string(),
    test_harness.output_file.display().to_string(),
  ]);

  set_up_test_files(test_harness.virtual_dir.path());

  test_harness.spawn_output();

  let found_files: Vec<String> = read_output(test_harness.output_file);

  for expected in vec![".", "..", "virtual_mod", "enabled_mods.json"] {
    assert!(
      found_files.iter().any(|found| found == expected),
      "expected file {expected:?} not in found {found_files:?}"
    )
  }
  assert_eq!(found_files.len(), 4)
}

#[ctest(crate::TESTS)]
fn relative_redirect() {
  let mut test_harness = TestHarness::new(FIND_FIRST_FILE);
  test_harness.set_args([
    Path::new("..")
      .join(test_harness.mount_dir.path())
      .join("*")
      .display()
      .to_string(),
    test_harness.output_file.display().to_string(),
  ]);
  test_harness.set_working_dir(test_harness.extra_dir.path().to_owned());

  set_up_test_files(test_harness.virtual_dir.path());

  test_harness.spawn_output();

  let found_files: Vec<String> = read_output(test_harness.output_file);

  for expected in vec![".", "..", "virtual_mod", "enabled_mods.json"] {
    assert!(
      found_files.iter().any(|found| found == expected),
      "expected file {expected:?} not in found {found_files:?}"
    )
  }
  assert_eq!(found_files.len(), 4)
}

#[ctest(crate::TESTS)]
fn no_redirect() {
  let mut test_harness = TestHarness::new(FIND_FIRST_FILE);
  test_harness.set_args([
    test_harness
      .extra_dir
      .path()
      .join("*")
      .display()
      .to_string(),
    test_harness.output_file.display().to_string(),
  ]);

  set_up_test_files(test_harness.extra_dir.path());

  test_harness.spawn_output();

  let found_files: Vec<String> = read_output(test_harness.output_file);

  for expected in vec![".", "..", "virtual_mod", "enabled_mods.json"] {
    assert!(
      found_files.iter().any(|found| found == expected),
      "expected file {expected:?} not in found {found_files:?}"
    )
  }
  assert_eq!(found_files.len(), 4)
}

fn set_up_test_files(path: impl AsRef<Path>) {
  let path = path.as_ref();
  std::fs::create_dir(path.join("virtual_mod")).unwrap();
  write_output(
    json!({
      "id": "virtual_mod",
      "name": "Virtual Mod",
      "author": "Virtual-kun",
      "utility": "true",
      "version": "6.6.6",
      "description": "This mod should only be loaded if the virtual filesystem has worked.",
      "gameVersion": "0.98a-RC8"
    }),
    path.join("virtual_mod/mod_info.json"),
  );
  write_output(
    json!({
      "enabledMods": [
        "virtual_mod"
      ]
    }),
    path.join("enabled_mods.json"),
  );
}
