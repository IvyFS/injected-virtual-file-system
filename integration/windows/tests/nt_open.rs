use integration_shared::{PrcoessOutputExt, TestHarness};
use proc_macros::ctest;

const NT_OPEN_CREATE_BIN: &str = env!("CARGO_BIN_EXE_NT_OPEN_CREATE");

#[ctest(crate::TESTS)]
fn open_virtual_only_dir() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  std::fs::create_dir(test_harness.virtual_expected()).unwrap();

  test_harness.set_args(["--is-dir", &test_harness.mount_expected_str(), "open"]);

  assert!(test_harness.spawn_output().status.success())
}

#[ctest(crate::TESTS)]
fn open_virtual_only_file() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  std::fs::write(test_harness.virtual_expected(), b"").unwrap();

  test_harness.set_args([&test_harness.mount_expected_str(), "open"]);

  assert!(test_harness.spawn_output().status.success())
}

#[ctest(crate::TESTS)]
fn should_fail() {
  let mut test_harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  test_harness.set_args([&test_harness.mount_expected_str(), "open"]);

  assert!(!test_harness.spawn_output().status.success())
}

#[ctest(crate::TESTS)]
fn open_virtual_only_dir_ignores_mount_file() {
  let mut harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  std::fs::create_dir(harness.virtual_expected()).unwrap();
  std::fs::write(harness.mount_expected(), b"").unwrap();

  harness.set_args(["--is-dir", &harness.mount_expected_str(), "open"]);

  let output = harness.spawn_output();

  assert!(output.status.success(), "{}", output.fmt_stdio());
}

#[ctest(crate::TESTS)]
fn open_virtual_only_file_ignores_mount_dir() {
  let mut harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  std::fs::write(harness.virtual_expected(), b"").unwrap();
  std::fs::create_dir(harness.mount_expected()).unwrap();

  harness.set_args([&harness.mount_expected_str(), "open"]);

  let output = harness.spawn_output();

  assert!(output.status.success(), "{}", output.fmt_stdio());
}

#[ctest(crate::TESTS)]
fn open_mount_only_dir_ignores_virtual_file() {
  let mut harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  std::fs::write(harness.virtual_expected(), b"").unwrap();
  std::fs::create_dir(harness.mount_expected()).unwrap();

  harness.set_args(["--is-dir", &harness.mount_expected_str(), "open"]);

  let output = harness.spawn_output();

  assert!(output.status.success(), "{}", output.fmt_stdio());
}

#[ctest(crate::TESTS)]
fn open_mount_only_file_ignores_virtual_dir() {
  let mut harness = TestHarness::new(NT_OPEN_CREATE_BIN);

  std::fs::create_dir(harness.virtual_expected()).unwrap();
  std::fs::write(harness.mount_expected(), b"").unwrap();

  harness.set_args([&harness.mount_expected_str(), "open"]);

  let output = harness.spawn_output();

  assert!(output.status.success(), "{}", output.fmt_stdio());
}
