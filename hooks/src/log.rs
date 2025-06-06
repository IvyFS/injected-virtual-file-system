use std::{
  error::Error,
  sync::{Mutex, OnceLock},
};

use interprocess::local_socket::Stream;
use shared_types::Message;

static SOCKET: OnceLock<Mutex<Stream>> = OnceLock::new();

pub fn init_logger(stream: Stream) {
  SOCKET.get_or_init(|| Mutex::new(stream));
}

pub fn log(msg: Message) {
  let mut socket = SOCKET.get().unwrap().lock().unwrap();
  msg.send(&mut *socket).unwrap();
}

pub fn log_lossy(msg: Message) {
  if let Some(mut socket) = SOCKET.get().and_then(|s| s.try_lock().ok()) {
    let _ = msg.send(&mut *socket);
  }
}

pub fn log_info(msg: impl ToString) {
  log(Message::DebugInfo(msg.to_string()));
}

pub fn log_error(err: impl Error) {
  log(Message::Error(err.to_string()));
}

macro_rules! trace {
  ($($tt:tt)*) => {
    if let Err(err) = (|| {
      $($tt)*
      Result::<_, shared_types::HookError>::Ok(())
    })() {
      crate::log::log_lossy(shared_types::Message::Error(err.to_string()))
    }
  };
}
pub(crate) use trace;
