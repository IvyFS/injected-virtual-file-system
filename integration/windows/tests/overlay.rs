use integration_shared::TestHarness;
use proc_macros::ctest;

const NT_QUERY_DIRECTORY_BIN: &str = env!("CARGO_BIN_EXE_NT_QUERY_DIRECTORY");

#[ctest(crate::TESTS)]
fn nt_query_directory_overlay_files() {
  let mut test_harness = TestHarness::new(NT_QUERY_DIRECTORY_BIN);

  std::fs::create_dir(test_harness.virtual_expected()).unwrap();
  std::fs::write(
    test_harness.virtual_expected().join("virtual.txt"),
    b"virtual file",
  )
  .unwrap();
  std::fs::create_dir(test_harness.mount_expected()).unwrap();
  std::fs::write(
    test_harness.mount_expected().join("mount.txt"),
    b"mounted file",
  )
  .unwrap();

  test_harness.set_args([
    &test_harness.mount_expected_str(),
    ".",
    "..",
    "virtual.txt",
    "mount.txt",
  ]);

  assert!(!test_harness.spawn_output().status.success());
}
