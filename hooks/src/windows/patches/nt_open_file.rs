use macros::{crabtime, generate_patch};
use win_api::{
  Wdk::Foundation::OBJECT_ATTRIBUTES,
  Win32::{
    Foundation::{HANDLE, NTSTATUS, STATUS_NO_SUCH_FILE},
    System::IO::IO_STATUS_BLOCK,
  },
};

use crate::{
  log::{logfmt_dbg, trace_expr},
  windows::os_types::handles::{DO_NOT_HOOK, HandleMap, ObjectAttributesExt},
};

generate_patch!(
  "NtOpenFile",
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
  objectattributes: *const OBJECT_ATTRIBUTES,
  iostatusblock: *mut IO_STATUS_BLOCK,
  shareaccess: u32,
  openoptions: u32,
) -> NTSTATUS {
  let res = trace_expr!(STATUS_NO_SUCH_FILE, unsafe {
    if shareaccess & DO_NOT_HOOK.0 == DO_NOT_HOOK.0 {
      logfmt_dbg!("Got DO_NOT_HOOK for path {:?}", objectattributes.path());
    }

    let original_fn = original();
    let res = original_fn(
      filehandle,
      desiredaccess,
      objectattributes,
      iostatusblock,
      shareaccess,
      openoptions,
    );

    if res.is_ok() {
      HandleMap::insert_by_object_attributes(*filehandle, objectattributes)?;
    }

    Ok(res)
  });

  res
}
