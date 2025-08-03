use std::{
  error::Error,
  sync::{
    LazyLock,
    atomic::{AtomicBool, AtomicU8},
  },
  time::Duration,
};

use crossbeam_queue::SegQueue;
use interprocess::local_socket::{GenericNamespaced, Stream, ToNsName, traits::Stream as _};
use shared_types::{
  config::hook::{HookLoggingConfig, HookLoggingVariant, IntoDiscriminant},
  message::Message, HookError,
};

pub(crate) use macros::*;

static MSG_QUEUE: LazyLock<SegQueue<Message>> = LazyLock::new(Default::default);
static LOGGING_VARIANT: AtomicU8 = AtomicU8::new(HookLoggingVariant::None as u8);

pub fn init_logging(config: HookLoggingConfig) {
  LOGGING_VARIANT.store(
    config.discriminant() as u8,
    std::sync::atomic::Ordering::Relaxed,
  );
  if let HookLoggingConfig::Ipc(socket_name) = config {
    let ns_name = socket_name.to_ns_name::<GenericNamespaced>().unwrap();
    let mut stream = Stream::connect(ns_name).unwrap();
    std::thread::spawn(move || {
      loop {
        if let Some(message) = MSG_QUEUE.pop()
          && let Err(err) = message.send(&mut stream)
        {
          eprintln!("{err:?}");
        }
      }
    });
  }
}

pub fn log(msg: Message) {
  let repr = LOGGING_VARIANT.load(std::sync::atomic::Ordering::Relaxed);
  match HookLoggingVariant::from_repr(repr) {
    Some(HookLoggingVariant::Ipc) => MSG_QUEUE.push(msg),
    Some(HookLoggingVariant::Stderr) => eprintln!("{msg}"),
    _ if let Message::Error(err) = msg => panic!("{err}"),
    _ => {}
  }
}

pub fn log_lossy(msg: Message) {
  log(msg);
}

pub fn log_info(msg: impl ToString) {
  log(Message::DebugInfo(msg.to_string()));
}

#[track_caller]
pub fn log_debug(msg: impl std::fmt::Debug) {
  let location = std::panic::Location::caller();
  log(Message::DebugInfo(format!(
    "Debug | {}:{} = {msg:?}",
    location.file(),
    location.line()
  )));
}

pub fn log_error(err: impl Error) {
  log(Message::Error(err.to_string()));
}

mod macros {
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

  macro_rules! trace_expr {
    ($default:expr, $($tt:tt)*) => {
      {
        #[allow(clippy::redundant_closure_call)]
        let res: Result::<_, shared_types::HookError> = (|| {
          $($tt)*
        })();
        match res {
          Ok(val) => val,
          Err(err) => {
            crate::log::log_lossy(shared_types::Message::Error(err.to_string()));
            return $default
          }
        }
      }
    };
  }

  macro_rules! logfmt_dbg {
    ($fmt_str:literal$($tt:tt)*) => {
      crate::log::log(shared_types::Message::DebugInfo(format!(concat!("Debug | ", file!(), ":", line!(), " = ", $fmt_str) $($tt)*)))
    };
  }

  pub(crate) use {logfmt_dbg, trace, trace_expr};
}
