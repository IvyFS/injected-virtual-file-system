use proc_macros::ctest;

use integration_shared::TestHarness;

const FOLDER_EDITOR: &str = env!("CARGO_BIN_FILE_INTEGRATION_SHARED_dir_edit");

#[ctest(crate::TESTS)]
fn create_directory_w() {
  let mut test_harness = TestHarness::new(FOLDER_EDITOR).parallel();

  if test_harness.mount_target.exists() {
    let _ = std::fs::remove_dir(&test_harness.mount_target);
  }
  if test_harness.virtual_target.exists() {
    let _ = std::fs::remove_dir(&test_harness.virtual_target);
  }

  test_harness
    .set_args([test_harness.mount_target.display().to_string()])
    .write_config_and_output();

  assert!(!test_harness.mount_target.exists());
  assert!(test_harness.virtual_target.exists());
}

#[ctest(crate::TESTS)]
fn remove_directory_w() {
  let mut test_harness = TestHarness::new(FOLDER_EDITOR).parallel();

  if !test_harness.mount_target.exists() {
    std::fs::create_dir(&test_harness.mount_target).unwrap();
  }
  if !test_harness.virtual_target.exists() {
    std::fs::create_dir(&test_harness.virtual_target).unwrap();
  }

  test_harness
    .set_args([
      test_harness.mount_target.display().to_string(),
      "delete".to_owned(),
    ])
    .write_config_and_output();

  assert!(test_harness.mount_target.exists());
  assert!(!test_harness.virtual_target.exists());
}
