use proc_macros::ctest;

use integration_shared::TestHarness;

const FOLDER_EDITOR: &str = env!("CARGO_BIN_FILE_INTEGRATION_SHARED_dir_edit");

#[ctest(crate::TESTS)]
fn create_directory_w() {
  let mut test_harness = TestHarness::new(FOLDER_EDITOR).parallel();

  if test_harness.mount_expected().exists() {
    let _ = std::fs::remove_dir(&test_harness.mount_expected());
  }
  if test_harness.virtual_expected().exists() {
    let _ = std::fs::remove_dir(&test_harness.virtual_expected());
  }

  test_harness
    .set_args([test_harness.mount_expected().display().to_string()])
    .spawn_output();

  assert!(!test_harness.mount_expected().exists());
  assert!(test_harness.virtual_expected().exists());
}

#[ctest(crate::TESTS)]
fn remove_directory_w() {
  let mut test_harness = TestHarness::new(FOLDER_EDITOR).parallel();

  if !test_harness.mount_expected().exists() {
    std::fs::create_dir(&test_harness.mount_expected()).unwrap();
  }
  if !test_harness.virtual_expected().exists() {
    std::fs::create_dir(&test_harness.virtual_expected()).unwrap();
  }

  test_harness
    .set_args([
      test_harness.mount_expected().display().to_string(),
      "delete".to_owned(),
    ])
    .spawn_output();

  // TODO: should deleting a virtual folder allow the target to see a mounted folder that might have been hidden by it?
  assert!(test_harness.mount_expected().exists());
  assert!(!test_harness.virtual_expected().exists());
}
