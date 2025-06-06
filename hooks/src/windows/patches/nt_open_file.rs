use macros::{crabtime, generate_patch};
use shared_types::Message;
use win_api::{
  Wdk::{Foundation::OBJECT_ATTRIBUTES, Storage::FileSystem::RtlInitUnicodeStringEx},
  Win32::{
    Foundation::{HANDLE, NTSTATUS, UNICODE_STRING},
    System::IO::IO_STATUS_BLOCK,
  },
};

use crate::log::*;
use crate::windows::handles::HandleMap;
pub use nt_open_file::*;

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
  trace!(unsafe {
    HandleMap::update_handles(*filehandle, objectattributes)?;
  });

  if let Some(attrs) = unsafe { objectattributes.as_ref() } {
    if let Some(object_name) = unsafe { attrs.ObjectName.as_ref() } {
      let name = unsafe { object_name.Buffer.to_string() }.unwrap();

      if name.contains("Starsector\\mods\\") {
        log(Message::DebugFileOpened(format!("file name {name}")));
      }
      if name.ends_with("Starsector\\mods\\*") && false {
        log_info(format!(
          "root dir null: {}",
          attrs.RootDirectory.0.is_null()
        ));

        let fake_name = windows_strings::w!(
          "\\??\\C:\\Users\\wanty\\Documents\\usvfs-rust\\demo\\examples\\target_folder\\*"
        );
        let mut unicode = UNICODE_STRING::default();
        let res = unsafe { RtlInitUnicodeStringEx(&mut unicode, fake_name) };
        log_info(format!("init unicode: {res:?}"));
        log_info(format!(
          "{}",
          unsafe { unicode.Buffer.to_string() }.unwrap()
        ));

        log_info(format!(
          "Attempting to return different file handle for {}",
          name.escape_debug()
        ));

        let fake_object_attrs = OBJECT_ATTRIBUTES {
          Length: attrs.Length,
          RootDirectory: HANDLE(std::ptr::null_mut()),
          ObjectName: &unicode,
          Attributes: attrs.Attributes,
          SecurityDescriptor: attrs.SecurityDescriptor,
          SecurityQualityOfService: attrs.SecurityQualityOfService,
        };

        let original = ORIGINAL_NT_OPEN_FILE
          .get()
          .expect("Get original NtOpenFile function ptr")
          .lock()
          .expect("Lock mutex on NtOpenFile ptr");

        let res = unsafe {
          original(
            filehandle,
            desiredaccess,
            &raw const fake_object_attrs,
            iostatusblock,
            shareaccess,
            openoptions,
          )
        };

        log_info(format!("res: {res:?}"));

        return res;
      }
    }
  }

  let original = ORIGINAL_NT_OPEN_FILE
    .get()
    .expect("Get original NtOpenFile function ptr")
    .lock()
    .expect("Lock mutex on NtOpenFile ptr");

  unsafe {
    original(
      filehandle,
      desiredaccess,
      objectattributes,
      iostatusblock,
      shareaccess,
      openoptions,
    )
  }
}
