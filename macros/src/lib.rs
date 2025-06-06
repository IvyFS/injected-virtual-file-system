#[cfg(target_os = "macos")]
mod darwin;
#[cfg(all(target_family = "unix", not(target_vendor = "apple")))]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
use darwin::*;
#[cfg(all(target_family = "unix", not(target_vendor = "apple")))]
use linux::*;
#[cfg(target_os = "windows")]
#[allow(unused_imports)]
pub use windows::*;

#[cfg(target_os = "windows")]
mod re_exports {
  pub use crabtime;
}

#[cfg(target_os = "windows")]
pub use re_exports::*;
