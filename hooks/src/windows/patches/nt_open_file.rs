use std::{ffi::OsString, path::PathBuf};

use proc_macros::patch_fn;
use win_api::{
  Wdk::Foundation::OBJECT_ATTRIBUTES,
  Win32::{
    Foundation::{HANDLE, NTSTATUS, STATUS_NO_SUCH_FILE},
    System::IO::IO_STATUS_BLOCK,
  },
};

use crate::{
  log::{logfmt_dbg, trace_expr},
  virtual_paths::windows::get_virtual_path,
  windows::helpers::{
    handles::{HANDLE_MAP, ObjectAttributesExt},
    object_attributes::RawObjectAttrsExt,
    retry_with,
  },
};

patch_fn!(
  NtOpenFile,
  (
    *mut HANDLE,
    u32,
    *const OBJECT_ATTRIBUTES,
    *mut IO_STATUS_BLOCK,
    u32,
    u32
  ) -> NTSTATUS,
  detour_nt_open_file
);

pub unsafe extern "system" fn detour_nt_open_file(
  filehandle: *mut HANDLE,
  desiredaccess: u32,
  original_attrs: *const OBJECT_ATTRIBUTES,
  iostatusblock: *mut IO_STATUS_BLOCK,
  shareaccess: u32,
  openoptions: u32,
) -> NTSTATUS {
  let res = trace_expr!(STATUS_NO_SUCH_FILE, unsafe {
    let path: PathBuf = original_attrs.path()?;
    let (attrs, reroute_guard) = if let Some(virtual_path) = get_virtual_path(&path)? {
      let attrs = original_attrs.reroute(virtual_path.path)?;
      (&raw const attrs.attrs, Some(attrs))
    } else {
      (original_attrs, None)
    };

    let res = original_nt_open_file(
      filehandle,
      desiredaccess,
      attrs,
      iostatusblock,
      shareaccess,
      openoptions,
    );

    logfmt_dbg!("{:x}", res.0);
    let (res, path, rerouted) = match reroute_guard {
      Some(_) if res.is_err() => {
        retry_with((res, OsString::new(), true), || {
          let res = original_nt_open_file(
            filehandle,
            desiredaccess,
            original_attrs,
            iostatusblock,
            shareaccess,
            openoptions,
          );

          res.is_ok().then(|| {
            (res, path.into_os_string(), false)
          })
        })
      }
      Some(reroute) => {
        logfmt_dbg!(
          "rerouting from {:?} to {:?}",
          path,
          reroute.unicode_path.string_buffer
        );
        (res, reroute.unicode_path.string_buffer.to_os_string(), true)
      }
      None => {
        (res, path.into_os_string(), false)
      }
    };

    if res.is_ok() {
      HANDLE_MAP.insert(filehandle, path, rerouted);
    }

    Ok(res)
  });

  res
}
