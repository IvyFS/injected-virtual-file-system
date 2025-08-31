use std::{path::Path, process::Command};

use proc_macros::ctest;
use tempdir::TempDir;

use integration_shared::{TestHarness, workspace_root};

const WORKSPACE_ROOT: &str = env!("CARGO_WORKSPACE_DIR");

pub(crate) fn clean_and_build() {
  let integration = Path::new(WORKSPACE_ROOT).join("integration");

  let output = Command::new(integration.join("assets/java-fs-demo/gradlew.bat"))
    .env("JAVA_HOME", integration.join("assets/jdk"))
    .current_dir(integration.join("assets/java-fs-demo"))
    .args(["clean", "jar"])
    .output()
    .unwrap();

  assert!(output.status.success());
}

#[test]
#[ignore]
fn sanity_test() {
  clean_and_build();

  let child = Command::new(env!("CARGO_BIN_FILE_INJECTOR"))
    .current_dir(WORKSPACE_ROOT)
    .arg("integration/assets/java-fs-demo-config.toml")
    .spawn()
    .unwrap();

  let output = child.wait_with_output().unwrap();
  assert!(output.status.success())
}

#[ctest(crate::TESTS)]
fn absolute_redirect() {
  clean_and_build();

  let assets_path = workspace_root().join("integration/assets");
  let jvm_path = assets_path.join("jdk/bin/java.exe");
  let mut test_harness = TestHarness::new(jvm_path.display()).expected_name("virtual_mod");

  std::fs::create_dir(test_harness.virtual_expected()).unwrap();
  std::fs::copy(
    workspace_root().join("integration/target_folder/virtual_mod/mod_info.json"),
    test_harness.virtual_expected().join("mod_info.json"),
  )
  .unwrap();

  let output = test_harness
    .set_args([
      "-classpath".to_owned(),
      assets_path
        .join("java-fs-demo/build/libs/java-fs-demo-0.0.1.jar")
        .display()
        .to_string(),
      "FsDemo".to_owned(),
      test_harness.mount_dir.path().display().to_string(),
    ])
    .write_config_and_output();

  let found = parse_java_bin_output(&output.stdout);

  assert_eq!(1, found.len(), "{found:?}");
  assert_eq!(test_harness.mount_expected_str(), found[0], "{found:?}");
}

#[ctest(crate::TESTS)]
fn relative_redirect() {
  clean_and_build();

  let temp_working_dir = TempDir::new("working_dir").unwrap();

  let assets_path = workspace_root().join("integration/assets");
  let jvm_path = assets_path.join("jdk/bin/java.exe");
  let mut test_harness = TestHarness::new(jvm_path.display())
    .with_working_dir(temp_working_dir.path())
    .expected_name("virtual_mod");

  std::fs::create_dir(test_harness.virtual_expected()).unwrap();
  std::fs::copy(
    workspace_root().join("integration/target_folder/virtual_mod/mod_info.json"),
    test_harness.virtual_expected().join("mod_info.json"),
  )
  .unwrap();

  let output = test_harness
    .set_args([
      "-classpath".to_owned(),
      assets_path
        .join("java-fs-demo/build/libs/java-fs-demo-0.0.1.jar")
        .display()
        .to_string(),
      "FsDemo".to_owned(),
      format!(
        "../{}",
        test_harness.mount_dir.path().file_name().unwrap().display()
      ),
    ])
    .write_config()
    .spawn_output();

  let found = parse_java_bin_output(&output.stdout);

  assert_eq!(1, found.len(), "{found:?}");
  assert_eq!(
    test_harness.mount_expected(),
    test_harness
      .mount_dir
      .path()
      .join(&found[0])
      .normalize_lexically()
      .unwrap(),
    "{found:?}"
  );
}

fn parse_java_bin_output(stdout: &[u8]) -> Vec<String> {
  let stdout = String::from_utf8_lossy(stdout)
    .replace(&['[', ']', '\r', '\n'], "")
    .replace("examples\\", "");

  stdout
    .split(",")
    .filter_map(|split| (!split.is_empty()).then(|| split.to_owned()))
    .collect()
}
