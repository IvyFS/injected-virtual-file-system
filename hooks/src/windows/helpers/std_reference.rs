use std::{mem::MaybeUninit, path::Path};

use std::os::windows::io::{AsRawHandle, HandleOrInvalid, OwnedHandle};
use win_types::PCWSTR;

use win_api::Win32::{
  Foundation::{
    ERROR_ALREADY_EXISTS, ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_PARAMETER, GENERIC_READ,
    GENERIC_WRITE, GetLastError, HANDLE, SetLastError, WIN32_ERROR,
  },
  Security::SECURITY_ATTRIBUTES,
  Storage::FileSystem::{
    CREATE_NEW, CreateFileW, FILE_ALLOCATION_INFO, FILE_CREATION_DISPOSITION,
    FILE_END_OF_FILE_INFO, FILE_FLAG_OPEN_REPARSE_POINT, FILE_FLAGS_AND_ATTRIBUTES,
    FILE_GENERIC_WRITE, FILE_SHARE_DELETE, FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    FILE_WRITE_DATA, FileAllocationInfo, FileEndOfFileInfo, GetFullPathNameW, OPEN_ALWAYS,
    OPEN_EXISTING, SECURITY_SQOS_PRESENT, SetFileInformationByHandle, TRUNCATE_EXISTING,
  },
};

#[derive(Clone, Debug)]
pub struct OpenOptions {
  // generic
  read: bool,
  write: bool,
  append: bool,
  truncate: bool,
  create: bool,
  create_new: bool,
  // system-specific
  custom_flags: u32,
  access_mode: Option<u32>,
  attributes: u32,
  share_mode: FILE_SHARE_MODE,
  security_qos_flags: FILE_FLAGS_AND_ATTRIBUTES,
  security_attributes: *mut SECURITY_ATTRIBUTES,
}

impl OpenOptions {
  pub fn new() -> OpenOptions {
    OpenOptions {
      // generic
      read: false,
      write: false,
      append: false,
      truncate: false,
      create: false,
      create_new: false,
      // system-specific
      custom_flags: 0,
      access_mode: None,
      share_mode: FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
      attributes: 0,
      security_qos_flags: FILE_FLAGS_AND_ATTRIBUTES(0),
      security_attributes: std::ptr::null_mut(),
    }
  }

  pub fn read(&mut self, read: bool) {
    self.read = read;
  }
  pub fn write(&mut self, write: bool) {
    self.write = write;
  }
  pub fn append(&mut self, append: bool) {
    self.append = append;
  }
  pub fn truncate(&mut self, truncate: bool) {
    self.truncate = truncate;
  }
  pub fn create(&mut self, create: bool) {
    self.create = create;
  }
  pub fn create_new(&mut self, create_new: bool) {
    self.create_new = create_new;
  }

  pub fn custom_flags(&mut self, flags: u32) {
    self.custom_flags = flags;
  }
  pub fn access_mode(&mut self, access_mode: u32) {
    self.access_mode = Some(access_mode);
  }
  pub fn share_mode(&mut self, share_mode: u32) {
    self.share_mode = FILE_SHARE_MODE(share_mode);
  }
  pub fn attributes(&mut self, attrs: u32) {
    self.attributes = attrs;
  }
  pub fn security_qos_flags(&mut self, flags: u32) {
    // We have to set `SECURITY_SQOS_PRESENT` here, because one of the valid flags we can
    // receive is `SECURITY_ANONYMOUS = 0x0`, which we can't check for later on.
    self.security_qos_flags = FILE_FLAGS_AND_ATTRIBUTES(flags) | SECURITY_SQOS_PRESENT;
  }
  pub fn security_attributes(&mut self, attrs: *mut SECURITY_ATTRIBUTES) {
    self.security_attributes = attrs;
  }

  fn get_access_mode(&self) -> std::io::Result<u32> {
    let rights = match (self.read, self.write, self.append, self.access_mode) {
      (.., Some(mode)) => Ok(mode),
      (true, false, false, None) => Ok(GENERIC_READ.0),
      (false, true, false, None) => Ok(GENERIC_WRITE.0),
      (true, true, false, None) => Ok(GENERIC_READ.0 | GENERIC_WRITE.0),
      (false, _, true, None) => Ok(FILE_GENERIC_WRITE.0 & !FILE_WRITE_DATA.0),
      (true, _, true, None) => Ok(GENERIC_READ.0 | (FILE_GENERIC_WRITE.0 & !FILE_WRITE_DATA.0)),
      (false, false, false, None) => Err(std::io::Error::from_raw_os_error(
        ERROR_INVALID_PARAMETER.0 as i32,
      )),
    }?;
    Ok(rights)
  }

  fn get_creation_mode(&self) -> std::io::Result<FILE_CREATION_DISPOSITION> {
    match (self.write, self.append) {
      (true, false) => {}
      (false, false) => {
        if self.truncate || self.create || self.create_new {
          return Err(std::io::Error::from_raw_os_error(
            ERROR_INVALID_PARAMETER.0 as i32,
          ));
        }
      }
      (_, true) => {
        if self.truncate && !self.create_new {
          return Err(std::io::Error::from_raw_os_error(
            ERROR_INVALID_PARAMETER.0 as i32,
          ));
        }
      }
    }

    Ok(match (self.create, self.truncate, self.create_new) {
      (false, false, false) => OPEN_EXISTING,
      (true, false, false) => OPEN_ALWAYS,
      (false, true, false) => TRUNCATE_EXISTING,
      // `CREATE_ALWAYS` has weird semantics so we emulate it using
      // `OPEN_ALWAYS` and a manual truncation step. See #115745.
      (true, true, false) => OPEN_ALWAYS,
      (_, _, true) => CREATE_NEW,
    })
  }

  fn get_flags_and_attributes(&self) -> FILE_FLAGS_AND_ATTRIBUTES {
    FILE_FLAGS_AND_ATTRIBUTES(self.custom_flags)
      | FILE_FLAGS_AND_ATTRIBUTES(self.attributes)
      | self.security_qos_flags
      | if self.create_new {
        FILE_FLAG_OPEN_REPARSE_POINT
      } else {
        FILE_FLAGS_AND_ATTRIBUTES(0)
      }
  }
}

pub fn open(path: &Path, opts: &OpenOptions) -> std::io::Result<OwnedHandle> {
  let path = maybe_verbatim(path)?;
  // SAFETY: maybe_verbatim returns null-terminated strings
  let path = widestring::U16CString::from_vec_truncate(path);
  open_native(path.as_ustr_with_nul(), opts)
}

fn open_native(path: &widestring::U16Str, opts: &OpenOptions) -> std::io::Result<OwnedHandle> {
  let creation = opts.get_creation_mode()?;
  let handle = unsafe {
    CreateFileW(
      PCWSTR::from_raw(path.as_ptr()),
      opts.get_access_mode()?,
      opts.share_mode,
      (!opts.security_attributes.is_null()).then_some(opts.security_attributes),
      creation,
      opts.get_flags_and_attributes(),
      None,
    )?
  };
  let handle = unsafe { HandleOrInvalid::from_raw_handle(handle.0) };
  if let Ok(handle) = OwnedHandle::try_from(handle) {
    // Manual truncation. See #115745.
    if opts.truncate && creation == OPEN_ALWAYS && unsafe { GetLastError() } == ERROR_ALREADY_EXISTS
    {
      // This first tries `FileAllocationInfo` but falls back to
      // `FileEndOfFileInfo` in order to support WINE.
      // If WINE gains support for FileAllocationInfo, we should
      // remove the fallback.
      let alloc = FILE_ALLOCATION_INFO { AllocationSize: 0 };
      unsafe {
        SetFileInformationByHandle(
          HANDLE(handle.as_raw_handle()),
          FileAllocationInfo,
          &raw const alloc as _,
          size_of::<FILE_ALLOCATION_INFO>() as u32,
        )
        .or_else(|_| {
          let eof = FILE_END_OF_FILE_INFO { EndOfFile: 0 };
          SetFileInformationByHandle(
            HANDLE(handle.as_raw_handle()),
            FileEndOfFileInfo,
            &raw const eof as _,
            size_of::<FILE_ALLOCATION_INFO>() as u32,
          )
        })
        .map_err(|err| std::io::Error::from_raw_os_error(err.code().0))?;
      }
    }
    Ok(handle)
  } else {
    Err(std::io::Error::last_os_error())
  }
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
