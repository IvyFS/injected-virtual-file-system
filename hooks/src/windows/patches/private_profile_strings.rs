use proc_macros::patch_fn;

use windows_strings::{PCSTR, PCWSTR, PSTR, PWSTR};

use crate::{
  log::trace_expr,
  virtual_paths::windows::{VirtualPathOption, get_virtual_path_or_ansi, get_virtual_path_or_wide},
};

patch_fn!(
  GetPrivateProfileStringA,
  (PCSTR, PCSTR, PCSTR, PSTR, u32, PCSTR) -> u32,
  detour_get_private_profile_string_a
);

unsafe extern "system" fn detour_get_private_profile_string_a(
  app_name: PCSTR,
  key_name: PCSTR,
  default: PCSTR,
  returned_string: PSTR,
  returned_len: u32,
  filename: PCSTR,
) -> u32 {
  trace_expr!(0, unsafe {
    let mut virtual_path_res = get_virtual_path_or_ansi(filename)?;

    Ok(original_get_private_profile_string_a(
      app_name,
      key_name,
      default,
      returned_string,
      returned_len,
      virtual_path_res.as_raw_or_original(),
    ))
  })
}

patch_fn!(
  GetPrivateProfileStringW,
  (PCWSTR, PCWSTR, PCWSTR, PWSTR, u32, PCWSTR) -> u32,
  detour_get_private_profile_string_w
);

unsafe extern "system" fn detour_get_private_profile_string_w(
  app_name: PCWSTR,
  key_name: PCWSTR,
  default: PCWSTR,
  returned_string: PWSTR,
  returned_len: u32,
  filename: PCWSTR,
) -> u32 {
  trace_expr!(0, unsafe {
    let mut virtual_path_res = get_virtual_path_or_wide(filename)?;

    Ok(original_get_private_profile_string_w(
      app_name,
      key_name,
      default,
      returned_string,
      returned_len,
      virtual_path_res.as_raw_or_original(),
    ))
  })
}
