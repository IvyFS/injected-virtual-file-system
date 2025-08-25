use std::io::Write;

use proc_macros::ctest;

use integration_shared::TestHarness;

const FILE_EDIT: &str = env!("CARGO_BIN_EXE_FILE_EDIT");

#[ctest(crate::TESTS)]
fn delete_file() {
  let mut test_harness = TestHarness::new(FILE_EDIT).parallel();

  let mut mount_target = std::fs::File::create(&test_harness.mount_expected()).unwrap();
  mount_target.write(b"").unwrap();
  mount_target.flush().unwrap();
  let mut virtual_target = std::fs::File::create(&test_harness.virtual_expected()).unwrap();
  virtual_target.write(b"").unwrap();
  virtual_target.flush().unwrap();

  test_harness
    .set_args([
      "delete".to_owned(),
      test_harness.mount_expected().display().to_string(),
    ])
    .write_config_and_output();

  assert!(test_harness.mount_expected().exists());
  assert!(!test_harness.virtual_expected().exists());
}

#[ctest(crate::TESTS)]
fn move_file_ansi() {
  let mut test_harness = TestHarness::new(FILE_EDIT).parallel();

  let mut mount_target = std::fs::File::create(&test_harness.mount_expected()).unwrap();
  mount_target.write(b"").unwrap();
  mount_target.flush().unwrap();
  let mut virtual_target = std::fs::File::create(&test_harness.virtual_expected()).unwrap();
  virtual_target.write(b"").unwrap();
  virtual_target.flush().unwrap();

  let mount_dest = test_harness.mount_dir.path().join("dest");
  let virtual_dest = test_harness.virtual_dir.path().join("dest");

  test_harness
    .set_args([
      "move-file-a".to_owned(),
      test_harness.mount_expected().display().to_string(),
      mount_dest.display().to_string(),
    ])
    .write_config_and_output();

  assert!(test_harness.mount_expected().exists());
  assert!(!mount_dest.exists());
  assert!(!test_harness.virtual_expected().exists());
  assert!(virtual_dest.exists());
}

#[ctest(crate::TESTS)]
fn move_file_wide() {
  let mut test_harness = TestHarness::new(FILE_EDIT).parallel();

  let mut mount_target = std::fs::File::create(&test_harness.mount_expected()).unwrap();
  mount_target.write(b"").unwrap();
  mount_target.flush().unwrap();
  let mut virtual_target = std::fs::File::create(&test_harness.virtual_expected()).unwrap();
  virtual_target.write(b"").unwrap();
  virtual_target.flush().unwrap();

  let mount_dest = test_harness.mount_dir.path().join("dest");
  let virtual_dest = test_harness.virtual_dir.path().join("dest");

  test_harness
    .set_args([
      "move-file-w".to_owned(),
      test_harness.mount_expected().display().to_string(),
      mount_dest.display().to_string(),
    ])
    .write_config_and_output();

  assert!(test_harness.mount_expected().exists());
  assert!(!mount_dest.exists());
  assert!(!test_harness.virtual_expected().exists());
  assert!(virtual_dest.exists());
}
