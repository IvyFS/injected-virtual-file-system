use std::time::Duration;

pub use bincode::error::{DecodeError, EncodeError};
use bincode::{decode_from_std_read, encode_into_std_write};

pub mod config;
mod errors;
pub mod unsafe_types;

pub use errors::*;

pub const DEFAULT_HEARTBEAT: Duration = Duration::from_millis(500);

#[derive(Debug, PartialEq, bincode::Encode, bincode::Decode, strum::Display)]
pub enum Message {
  #[strum(to_string = "Info: {0}")]
  DebugInfo(String),
  #[strum(to_string = "Info: Running default pass-through intercept for {0}")]
  DebugDefaultIntercept(String),
  #[strum(to_string = "Info: Target program modules\n-----\n{0}\n-----")]
  DebugGetModules(String),
  #[strum(to_string = "Info: Opened {0}")]
  DebugFileOpened(String),
  Trace {
    file: String,
    line: u32,
    function: String,
    message: String,
  },
  #[strum(to_string = "Finished patching target process")]
  FinishedPatching,
  ShutdownCountdown(usize),
  ShutdownFinal,
  #[strum(to_string = "Error: Hook error: {0}")]
  Error(String),
}

impl Message {
  pub fn send(self, writer: &mut impl std::io::Write) -> Result<usize, EncodeError> {
    encode_into_std_write(self, writer, bincode::config::standard())
  }

  pub fn recv(reader: &mut impl std::io::Read) -> Result<Message, DecodeError> {
    decode_from_std_read(reader, bincode::config::standard())
  }
}
