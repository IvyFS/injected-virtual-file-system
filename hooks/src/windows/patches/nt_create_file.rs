use std::{ffi::c_void, ops::ControlFlow};

use proc_macros::patch_fn;
use shared_types::HookError;
use win_api::{
  Wdk::Storage::FileSystem::{
    FILE_CREATE, FILE_DIRECTORY_FILE, FILE_OPEN_IF, FILE_OVERWRITE_IF, FILE_SUPERSEDE,
  },
  Win32::Foundation::STATUS_OBJECT_NAME_EXISTS,
};
pub use win_api::{
  Wdk::{
    Foundation::OBJECT_ATTRIBUTES,
    Storage::FileSystem::{NTCREATEFILE_CREATE_DISPOSITION, NTCREATEFILE_CREATE_OPTIONS},
  },
  Win32::{
    Foundation::{HANDLE, NTSTATUS},
    Storage::FileSystem::{FILE_ACCESS_RIGHTS, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_MODE},
    System::IO::IO_STATUS_BLOCK,
  },
};

use crate::{
  extension_traits::{ControlContinues, ControlFlowExt, ResultIntoControlFlow},
  log::trace_inspect,
  virtual_paths::windows::get_virtual_path,
  windows::helpers::{
    handles::{HANDLE_MAP, ObjectAttributesExt},
    object_attributes::RawObjectAttrsExt,
    unhooked_fs,
  },
};

patch_fn!(
  NtCreateFile,
  (
    *mut HANDLE,
    FILE_ACCESS_RIGHTS,
    *const OBJECT_ATTRIBUTES,
    *mut IO_STATUS_BLOCK,
    *const i64,
    FILE_FLAGS_AND_ATTRIBUTES,
    FILE_SHARE_MODE,
    NTCREATEFILE_CREATE_DISPOSITION,
    NTCREATEFILE_CREATE_OPTIONS,
    *const c_void,
    u32
  ) -> NTSTATUS,
  detour_nt_create_file
);

/**
| Disposition       | Original                                                                | Hooked                                              |
| ----------------- | ----------------------------------------------------------------------- | --------------------------------------------------- |
| FILE_SUPERSEDE    | If the file already exists then replace it with the given file.         | If file exists in mounted directory but not virtual |
|                   | If it does not then create the given file.                              | supersede the mounted file. Otherwise no change.    |
|                   |                                                                         |                                                     |
| FILE_CREATE       | If the file already exists then fail the request and do not create      | If file exists in mounted directory fail early.     |
|                   | or open the given file. If it does not then create the given file.      | Otherwise no change.                                |
|                   |                                                                         |                                                     |
| FILE_OPEN         | If the file already exists then open it instead of creating a new file. | No change                                           |
|                   | If it does not then fail the request and do not create a new file.      |                                                     |
|                   |                                                                         |                                                     |
| FILE_OPEN_IF      | If the file already exists then open it.                                | If file exists in mounted directory but not virtual |
|                   | If it does not then create the given file.                              | open the mounted file. Otherwise no change.         |
|                   |                                                                         |                                                     |
| FILE_OVERWRITE    | If the file already exists then open it and overwrite it.               | No change                                           |
|                   | If it does not then fail the request.                                   |                                                     |
|                   |                                                                         |                                                     |
| FILE_OVERWRITE_IF | If the file already exists then open it and overwrite it.               | If file exists in mounted directory but not virtual |
|                   | If it does not then create the given file.                              | overwrite the mounted file. Otherwise no change.    |
|                   |                                                                         |                                                     |
 */
pub(crate) unsafe extern "system" fn detour_nt_create_file(
  handle: *mut HANDLE,
  _1: FILE_ACCESS_RIGHTS,
  original_attrs: *const OBJECT_ATTRIBUTES,
  _3: *mut IO_STATUS_BLOCK,
  _4: *const i64,
  flags_and_attributes: FILE_FLAGS_AND_ATTRIBUTES,
  share_mode: FILE_SHARE_MODE,
  disposition: NTCREATEFILE_CREATE_DISPOSITION,
  options: NTCREATEFILE_CREATE_OPTIONS,
  _9: *const c_void,
  _10: u32,
) -> NTSTATUS {
  let is_dir = options.contains(FILE_DIRECTORY_FILE);
  let path = unsafe { original_attrs.path() };

  let reroute_attrs = trace_inspect!(unsafe {
    let path = path.as_ref().map_err(Clone::clone)?;
    let virtual_path = get_virtual_path(path)?.ok_or(HookError::NoVirtualPath)?;
    original_attrs.reroute(virtual_path.path)
  });

  let reroute_attrs = match disposition {
    _ if let Err(HookError::NoVirtualPath) = reroute_attrs => None,
    FILE_CREATE => match unhooked_fs::exists(unsafe { original_attrs.as_ref().unwrap() }, is_dir) {
      Ok(true) => return STATUS_OBJECT_NAME_EXISTS,
      Ok(false) => Some(reroute_attrs.unwrap().continues()),
      Err(err_status) => return err_status,
    },
    FILE_SUPERSEDE | FILE_OPEN_IF | FILE_OVERWRITE_IF => {
      let path_check = unhooked_fs::exists(unsafe { original_attrs.as_ref().unwrap() }, is_dir)
        .and_then(|mount_exists| {
          if mount_exists {
            unhooked_fs::exists(&reroute_attrs.as_ref().unwrap().attrs, is_dir)
              .map(|virtual_exists| mount_exists && !virtual_exists)
          } else {
            Ok(false)
          }
        });
      match path_check {
        Ok(true) => None,
        Ok(false) => Some(reroute_attrs.unwrap().continues()),
        Err(err_status) => return err_status,
      }
    }
    _ => reroute_attrs.map_continues().ok(),
  };

  let hook_res = reroute_attrs.map(|reroute_attrs| {
    reroute_attrs.map_either(|reroute_attrs| {
      let attrs = &raw const reroute_attrs.attrs;
      let res = unsafe {
        original_nt_create_file(
          handle,
          _1,
          attrs,
          _3,
          _4,
          flags_and_attributes,
          share_mode,
          disposition,
          options,
          _9,
          _10,
        )
      };

      if res.is_ok() {
        Ok((reroute_attrs, res))
      } else {
        Err(res)
      }
    })
  });

  match hook_res {
    Some(
      ControlFlow::Continue(Ok((owned_attrs, res))) | ControlFlow::Break(Ok((owned_attrs, res))),
    ) => {
      HANDLE_MAP.insert(
        handle,
        owned_attrs.unicode_path.string_buffer.to_os_string(),
        true,
      );
      res
    }
    Some(ControlFlow::Break(Err(err_status))) => err_status,
    None | Some(ControlFlow::Continue(Err(_))) => {
      let res = unsafe {
        original_nt_create_file(
          handle,
          _1,
          original_attrs,
          _3,
          _4,
          flags_and_attributes,
          share_mode,
          disposition,
          options,
          _9,
          _10,
        )
      };
      if res.is_ok()
        && let Ok(path) = path
      {
        HANDLE_MAP.insert(handle, path, false);
      }
      res
    }
  }
}
