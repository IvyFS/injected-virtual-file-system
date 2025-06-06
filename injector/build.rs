use std::{
  env::{
    consts::{DLL_PREFIX, DLL_SUFFIX},
    var,
  },
  path::PathBuf,
};

fn main() {
  let workspace_root = var("CARGO_WORKSPACE_DIR").expect("Get CARGO_WORKSPACE_DIR env var");
  let profile = var("PROFILE").expect("Get PROFILE");

  let dylib_path = {
    let mut dylib_path = PathBuf::from(workspace_root);
    dylib_path.push("target");
    dylib_path.push(profile);
    dylib_path.push(format!("deps/{}agent{}", DLL_PREFIX, DLL_SUFFIX));
    dylib_path.to_string_lossy().into_owned()
  };
  println!("cargo::rustc-env=DYLIB_PATH={dylib_path}")
}
