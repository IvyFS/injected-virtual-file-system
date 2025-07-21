use proc_macros::patch_fn;
use win_api::Win32::Foundation::HANDLE;

use crate::windows::{os_types::handles::HANDLE_MAP, patches::QUERY_MAP};

patch_fn!(NtClose, (HANDLE), nt_close_detour);

pub(crate) unsafe extern "system" fn nt_close_detour(handle: HANDLE) {
  if let Some((_, query_handle)) = QUERY_MAP.remove_query(handle) {
    if query_handle.0 != handle.0 {
      unsafe { nt_close_detour(query_handle.into()) };
    }
  }
  HANDLE_MAP.remove_by_handle(handle);

  unsafe { nt_close_unhooked(handle) }
}

pub(crate) unsafe fn nt_close_unhooked(handle: HANDLE) {
  unsafe { nt_close::original_nt_close(handle) }
}
