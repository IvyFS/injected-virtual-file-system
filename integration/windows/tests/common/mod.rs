use win_api::Win32::{Foundation::HANDLE, Storage::FileSystem::GetFinalPathNameByHandleW};

pub unsafe fn path_from_handle(handle: HANDLE) -> String {
  unsafe {
    const LEN: usize = 1024;
    let mut buffer = [0; LEN];
    let len = GetFinalPathNameByHandleW(handle, &mut buffer, Default::default());
    if len != 0 && len < LEN as u32 {
      String::from_utf16(&buffer[0..(len as usize)]).unwrap()
    } else {
      panic!("Returned path longer than buffer: {len}");
    }
  }
}
