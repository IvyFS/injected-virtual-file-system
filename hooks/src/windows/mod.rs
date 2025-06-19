use std::{collections::HashMap, sync::LazyLock};

pub(crate) mod os_types;
pub(crate) mod patches;

use patches::*;

pub static HOOK_TARGETS: LazyLock<
  HashMap<(&'static str, Option<&'static str>), Vec<(&'static str, Option<FuncPatcher>)>>,
> = LazyLock::new(|| {
  const KERNEL_MODULES: (&str, Option<&str>) = ("kernelbase.dll", Some("kernel32.dll"));

  let mut targets = HashMap::from([(KERNEL_MODULES, WIN32_TARGETS.to_vec())]);
  if windows_version::OsVersion::current().major >= 8 {
    targets
      .get_mut(&KERNEL_MODULES)
      .expect("Get kernel targets")
      .extend(WIN8_PLUS_WIN32_TARGETS);
  }
  targets.insert(("ntdll.dll", None), NT_TARGETS.to_vec());

  targets
});
