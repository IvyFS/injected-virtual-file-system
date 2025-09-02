use integration_shared::TestHarness;
use proc_macros::ctest;

const NT_QUERY_DIRECTORY_BIN: &str = env!("CARGO_BIN_EXE_NT_QUERY_DIRECTORY");

#[ctest(crate::TESTS)]
fn query_directory_empty() {
  let mut test_harness = TestHarness::new(NT_QUERY_DIRECTORY_BIN);

  std::fs::create_dir(test_harness.virtual_expected()).unwrap();

  test_harness.set_args([&test_harness.mount_expected_str(), ".", ".."]);

  assert!(test_harness.spawn_output().status.success())
}

#[ctest(crate::TESTS)]
fn query_directory_multiple() {
  let mut test_harness = TestHarness::new(NT_QUERY_DIRECTORY_BIN);

  std::fs::create_dir(test_harness.virtual_expected()).unwrap();
  std::fs::create_dir(test_harness.virtual_expected().join("virtual_mod")).unwrap();
  std::fs::write(
    test_harness.virtual_expected().join("enabled_mods.json"),
    b"",
  )
  .unwrap();

  test_harness.set_args([
    &test_harness.mount_expected_str(),
    ".",
    "..",
    "virtual_mod",
    "enabled_mods.json",
  ]);

  assert!(test_harness.spawn_output().status.success());
}

#[ctest(crate::TESTS)]
fn query_directory_should_fail() {
  let mut test_harness = TestHarness::new(NT_QUERY_DIRECTORY_BIN);

  std::fs::create_dir(test_harness.virtual_expected()).unwrap();

  test_harness.set_args([
    &test_harness.mount_expected_str(),
    ".",
    "..",
    "virtual_mod",
    "enabled_mods.json",
  ]);

  assert!(!test_harness.spawn_output().status.success());
}
