use integration_shared::TestHarness;
use proc_macros::ctest;

const NT_OPEN_CREATE_BIN: &str = env!("CARGO_BIN_EXE_NT_OPEN_CREATE");

#[ctest(crate::TESTS)]
fn open_existing_dir_in_virtual_fs() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  std::fs::create_dir(&test_harness.virtual_expected()).unwrap();

  test_harness.set_args(["--is-dir", &test_harness.mount_expected_str(), "create"]);

  assert!(test_harness.write_config_and_output().status.success())
}

#[ctest(crate::TESTS)]
fn should_fail() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  test_harness.set_args(["--is-dir", &test_harness.mount_expected_str(), "create"]);

  assert!(!test_harness.write_config_and_output().status.success())
}

#[ctest(crate::TESTS)]
fn mkdir_creates_dir_in_virtual_fs() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  test_harness.set_args([
    "--is-dir",
    &test_harness.virtual_expected_str(),
    "create",
    "--create-not-exists",
  ]);

  assert!(test_harness.write_config_and_output().status.success());
  assert!(test_harness.virtual_expected().is_dir());
}

#[ctest(crate::TESTS)]
fn open_existing_file_in_virtual_fs() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  std::fs::write(&test_harness.virtual_expected(), b"").unwrap();

  test_harness.set_args([&test_harness.mount_expected_str(), "create"]);

  assert!(test_harness.write_config_and_output().status.success())
}

#[ctest(crate::TESTS)]
fn mk_file_creates_file_in_virtual_fs() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  test_harness.set_args([
    &test_harness.mount_expected_str(),
    "create",
    "--create-not-exists",
  ]);

  assert!(test_harness.write_config_and_output().status.success());
  assert!(test_harness.virtual_expected().is_file());
}
