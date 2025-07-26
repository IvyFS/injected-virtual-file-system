use std::{path::Path, process::Command};

const WORKSPACE_ROOT: &str = env!("CARGO_WORKSPACE_DIR");
const INJECTOR: &str = env!("CARGO_BIN_FILE_INJECTOR");

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

pub(crate) fn java_list_dirs(capture: bool) -> Vec<String> {
  let mut command = Command::new(INJECTOR);
  command
    .current_dir(WORKSPACE_ROOT)
    .arg("integration/assets/java-fs-demo-config.toml");

  if capture {
    let output = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout)
      .replace(&['[', ']', '\r', '\n'], "")
      .replace("examples\\", "");

    stdout.split(",").map(ToOwned::to_owned).collect()
  } else {
    let mut child = command.spawn().unwrap();

    assert!(child.wait().unwrap().success());

    Vec::new()
  }
}

#[test]
fn sanity_test() {
  clean_and_build();

  let mut child = Command::new(INJECTOR)
    .current_dir(WORKSPACE_ROOT)
    .arg("integration/assets/java-fs-demo-config.toml")
    .spawn()
    .unwrap();

  assert!(child.wait().unwrap().success())
}

#[test]
fn absolute_redirect() {
  clean_and_build();

  let found_files = dbg!(java_list_dirs(true));

  for expected in vec!["virtual_mod"] {
    assert!(
      found_files.iter().any(|found| found == expected),
      "expected file {expected:?} not in found {found_files:?}"
    )
  }
  assert_eq!(found_files.len(), 1)
}
