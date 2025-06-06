use std::{collections::HashMap, sync::LazyLock};

use frida_gum::{Gum, Module};

mod function_targets;
mod handles;
mod patches;

use function_targets::*;
use shared_types::HookError;

type FuncPatcher = fn(&Gum, &Module, &str) -> Result<(), HookError>;

pub static HOOK_TARGETS: LazyLock<
  HashMap<(&'static str, Option<&'static str>), Vec<(&'static str, Option<FuncPatcher>)>>,
> = LazyLock::new(|| {
  const KERNEL_MODULES: (&str, Option<&str>) = ("kernelbase.dll", Some("kernel32.dll"));

  let mut targets = HashMap::from([(KERNEL_MODULES, KERNEL_TARGETS.to_vec())]);
  if windows_version::OsVersion::current().major >= 8 {
    targets
      .get_mut(&KERNEL_MODULES)
      .expect("Get kernel targets")
      .extend(WIN_8_OR_LATER_KERNEL_TARGETS);
  }
  targets.insert(("ntdll.dll", None), NT_TARGETS.to_vec());

  targets
});
