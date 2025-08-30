use integration_shared::TestHarness;
use proc_macros::ctest;

const NT_OPEN_CREATE_BIN: &str = env!("CARGO_BIN_EXE_NT_OPEN_CREATE");

#[ctest(crate::TESTS)]
fn nt_open_existing_virtual_dir() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  std::fs::create_dir(test_harness.virtual_expected()).unwrap();

  test_harness.set_args(["--is-dir", &test_harness.mount_expected_str(), "open"]);

  assert!(test_harness.write_config_and_output().status.success())
}

#[ctest(crate::TESTS)]
fn nt_open_existing_virtual_file() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  std::fs::write(test_harness.virtual_expected(), b"").unwrap();

  test_harness.set_args([&test_harness.mount_expected_str(), "open"]);

  assert!(test_harness.write_config_and_output().status.success())
}

#[ctest(crate::TESTS)]
fn should_fail() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  test_harness.set_args([&test_harness.mount_expected_str(), "open"]);

  assert!(!test_harness.write_config_and_output().status.success())
}
