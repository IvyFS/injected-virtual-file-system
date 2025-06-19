#![feature(maybe_uninit_slice)]

use frida_gum::{Gum, Process};
use interprocess::local_socket::Stream;
use shared_types::{HookError, Message, config::VirtualFsConfig};

pub mod extension_traits;
#[allow(unused)]
mod log;
mod raw_ptr;
mod virtual_paths;

#[cfg(target_os = "macos")]
mod darwin;
#[cfg(all(target_family = "unix", not(target_vendor = "apple")))]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(target_os = "macos")]
use darwin::*;
#[cfg(all(target_family = "unix", not(target_vendor = "apple")))]
use linux::*;

use crate::{
  log::{init_logger, log},
  virtual_paths::{MOUNT_POINT, VIRTUAL_ROOT},
};

struct Sealed;

pub struct Patcher<'a> {
  _sealed: Sealed,
  gum: &'a Gum,
}

impl<'a> Patcher<'a> {
  pub fn init(gum: &'a Gum, socket: Option<Stream>, config: VirtualFsConfig) -> Patcher<'a> {
    init_logger(socket);
    *MOUNT_POINT.write().unwrap() = config.mount_point;
    *VIRTUAL_ROOT.write().unwrap() = config.virtual_root;

    #[cfg(windows)]
    windows::os_types::handles::init_handles();

    Patcher {
      _sealed: Sealed,
      gum,
    }
  }

  pub fn log(&self, msg: Message) {
    log(msg);
  }

  pub fn patch_functions(&self) -> Result<(), HookError> {
    let process = Process::obtain(&self.gum);

    for ((module, backup_module), patches) in &*windows::HOOK_TARGETS {
      let module = process.find_module_by_name(module);
      let backup_module = backup_module.and_then(|m| process.find_module_by_name(m));

      for (function_name, patcher) in patches.iter().filter_map(|(f, p)| Some(f).zip(*p)) {
        let res = if let Some(module) = &module {
          patcher(&self.gum, &module, &function_name)
        } else {
          Ok(())
        };

        // Only try patching the backup if:
        // - We didn't find the main module
        // - We couldn't find the target function in the main module
        if module.is_none() || matches!(res, Err(HookError::FunctionNotFound { .. })) {
          if let Some(backup) = &backup_module {
            patcher(&self.gum, backup, &function_name)?;
          }
        } else {
          // Otherwise:
          // - The patch succeeded, which is fine
          // - Or the patch failed for an actually fatal reason, which we should raise
          res?
        }
      }
    }

    Ok(())
  }
}
