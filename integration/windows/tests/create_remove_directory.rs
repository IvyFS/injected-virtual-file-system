use proc_macros::ctest;

use integration_shared::TestHarness;

const FOLDER_EDITOR: &str = env!("CARGO_BIN_FILE_INTEGRATION_SHARED_dir_edit");

#[ctest(crate::TESTS)]
fn creates_directory_in_virtual_dir_when_no_conflict() {
  let mut test_harness = TestHarness::new(FOLDER_EDITOR).parallel();

  let output = test_harness
    .set_args([test_harness.mount_expected().display().to_string()])
    .spawn_output();
  assert!(output.status.success());

  assert!(!test_harness.mount_expected().exists());
  assert!(test_harness.virtual_expected().exists());
}

#[ctest(crate::TESTS)]
fn fails_directory_creation_when_folder_exists_only_in_mounted_dir() {
  let mut test_harness = TestHarness::new(FOLDER_EDITOR).parallel();

  std::fs::create_dir(test_harness.mount_expected()).unwrap();

  let output = test_harness
    .set_args([test_harness.mount_expected().display().to_string()])
    .spawn_output();
  assert!(!output.status.success());

  assert!(!test_harness.virtual_expected().exists());
}

#[ctest(crate::TESTS)]
fn remove_directory_w() {
  let mut test_harness = TestHarness::new(FOLDER_EDITOR).parallel();

  std::fs::create_dir(&test_harness.mount_expected()).unwrap();
  std::fs::create_dir(&test_harness.virtual_expected()).unwrap();

  let output = test_harness
    .set_args([
      test_harness.mount_expected().display().to_string(),
      "delete".to_owned(),
    ])
    .spawn_output();
  assert!(output.status.success());

  assert!(!test_harness.mount_expected().exists());
  assert!(!test_harness.virtual_expected().exists());
}
