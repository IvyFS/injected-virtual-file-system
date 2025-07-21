use std::{
  mem::MaybeUninit,
  path::{Component, Path},
};

use win_api::Win32::{
  Foundation::{ERROR_INSUFFICIENT_BUFFER, GetLastError, SetLastError, WIN32_ERROR},
  Storage::FileSystem::GetFullPathNameW,
};
use win_types::PCWSTR;

pub const NT_PATH_PREFIX: &str = "\\??\\";

pub fn strip_nt_prefix(path: &impl AsRef<Path>) -> &Path {
  let path = path.as_ref();
  path.strip_prefix(NT_PATH_PREFIX).unwrap_or(path)
}

/// Checks if path fragment is relative
pub fn fragment_is_relative(path: impl AsRef<Path>) -> bool {
  path
    .as_ref()
    .components()
    .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}

pub(crate) fn maybe_verbatim(path: &Path) -> std::io::Result<Vec<u16>> {
  let path = widestring::U16CString::from_os_str_truncate(path);
  get_long_path(path.into_vec_with_nul(), true)
}

pub(crate) fn get_long_path(
  mut path: Vec<u16>,
  prefer_verbatim: bool,
) -> std::io::Result<Vec<u16>> {
  // Normally the MAX_PATH is 260 UTF-16 code units (including the NULL).
  // However, for APIs such as CreateDirectory[1], the limit is 248.
  //
  // [1]: https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectorya#parameters
  const LEGACY_MAX_PATH: usize = 248;
  // UTF-16 encoded code points, used in parsing and building UTF-16 paths.
  // All of these are in the ASCII range so they can be cast directly to `u16`.
  const SEP: u16 = b'\\' as _;
  const ALT_SEP: u16 = b'/' as _;
  const QUERY: u16 = b'?' as _;
  const COLON: u16 = b':' as _;
  const DOT: u16 = b'.' as _;
  const U: u16 = b'U' as _;
  const N: u16 = b'N' as _;
  const C: u16 = b'C' as _;

  // \\?\
  const VERBATIM_PREFIX: &[u16] = &[SEP, SEP, QUERY, SEP];
  // \??\
  const NT_PREFIX: &[u16] = &[SEP, QUERY, QUERY, SEP];
  // \\?\UNC\
  const UNC_PREFIX: &[u16] = &[SEP, SEP, QUERY, SEP, U, N, C, SEP];

  if path.starts_with(VERBATIM_PREFIX) || path.starts_with(NT_PREFIX) || path == [0] {
    // Early return for paths that are already verbatim or empty.
    return Ok(path);
  } else if path.len() < LEGACY_MAX_PATH {
    // Early return if an absolute path is less < 260 UTF-16 code units.
    // This is an optimization to avoid calling `GetFullPathNameW` unnecessarily.
    match path.as_slice() {
      // Starts with `D:`, `D:\`, `D:/`, etc.
      // Does not match if the path starts with a `\` or `/`.
      [drive, COLON, 0] | [drive, COLON, SEP | ALT_SEP, ..]
        if *drive != SEP && *drive != ALT_SEP =>
      {
        return Ok(path);
      }
      // Starts with `\\`, `//`, etc
      [SEP | ALT_SEP, SEP | ALT_SEP, ..] => return Ok(path),
      _ => {}
    }
  }

  // Firstly, get the absolute path using `GetFullPathNameW`.
  // https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfullpathnamew
  let lpfilename = path.as_ptr();
  fill_utf16_buf(
    // SAFETY: `fill_utf16_buf` ensures the `buffer` and `size` are valid.
    // `lpfilename` is a pointer to a null terminated string that is not
    // invalidated until after `GetFullPathNameW` returns successfully.
    |buffer| unsafe { GetFullPathNameW(PCWSTR::from_raw(lpfilename), Some(buffer), None) },
    |mut absolute| {
      path.clear();

      // Only prepend the prefix if needed.
      if prefer_verbatim || absolute.len() + 1 >= LEGACY_MAX_PATH {
        // Secondly, add the verbatim prefix. This is easier here because we know the
        // path is now absolute and fully normalized (e.g. `/` has been changed to `\`).
        let prefix = match absolute {
          // C:\ => \\?\C:\
          [_, COLON, SEP, ..] => VERBATIM_PREFIX,
          // \\.\ => \\?\
          [SEP, SEP, DOT, SEP, ..] => {
            absolute = &absolute[4..];
            VERBATIM_PREFIX
          }
          // Leave \\?\ and \??\ as-is.
          [SEP, SEP, QUERY, SEP, ..] | [SEP, QUERY, QUERY, SEP, ..] => &[],
          // \\ => \\?\UNC\
          [SEP, SEP, ..] => {
            absolute = &absolute[2..];
            UNC_PREFIX
          }
          // Anything else we leave alone.
          _ => &[],
        };

        path.reserve_exact(prefix.len() + absolute.len() + 1);
        path.extend_from_slice(prefix);
      } else {
        path.reserve_exact(absolute.len() + 1);
      }
      path.extend_from_slice(absolute);
      path.push(0);
    },
  )?;
  Ok(path)
}

pub fn fill_utf16_buf<F1, F2, T>(mut f1: F1, f2: F2) -> std::io::Result<T>
where
  F1: FnMut(&mut [u16]) -> u32,
  F2: FnOnce(&[u16]) -> T,
{
  // Start off with a stack buf but then spill over to the heap if we end up
  // needing more space.
  //
  // This initial size also works around `GetFullPathNameW` returning
  // incorrect size hints for some short paths:
  // https://github.com/dylni/normpath/issues/5
  let mut stack_buf: [MaybeUninit<u16>; 512] = [MaybeUninit::uninit(); 512];
  let mut heap_buf: Vec<MaybeUninit<u16>> = Vec::new();
  unsafe {
    let mut n = stack_buf.len();
    loop {
      let buf = if n <= stack_buf.len() {
        &mut stack_buf[..]
      } else {
        let extra = n - heap_buf.len();
        heap_buf.reserve(extra);
        // We used `reserve` and not `reserve_exact`, so in theory we
        // may have gotten more than requested. If so, we'd like to use
        // it... so long as we won't cause overflow.
        n = heap_buf.capacity().min(u32::MAX as usize);
        // Safety: MaybeUninit<u16> does not need initialization
        heap_buf.set_len(n);
        &mut heap_buf[..]
      };

      // This function is typically called on windows API functions which
      // will return the correct length of the string, but these functions
      // also return the `0` on error. In some cases, however, the
      // returned "correct length" may actually be 0!
      //
      // To handle this case we call `SetLastError` to reset it to 0 and
      // then check it again if we get the "0 error value". If the "last
      // error" is still 0 then we interpret it as a 0 length buffer and
      // not an actual error.
      SetLastError(WIN32_ERROR(0));
      let k = match f1(buf.assume_init_mut()) {
        0 if GetLastError().0 == 0 => 0,
        0 => return Err(std::io::Error::last_os_error()),
        n => n,
      } as usize;
      if k == n && GetLastError() == ERROR_INSUFFICIENT_BUFFER {
        n = n.saturating_mul(2).min(u32::MAX as usize);
      } else if k > n {
        n = k;
      } else if k == n {
        // It is impossible to reach this point.
        // On success, k is the returned string length excluding the null.
        // On failure, k is the required buffer length including the null.
        // Therefore k never equals n.
        unreachable!();
      } else {
        // Safety: First `k` values are initialized.
        let slice: &[u16] = buf[..k].assume_init_ref();
        return Ok(f2(slice));
      }
    }
  }
}
