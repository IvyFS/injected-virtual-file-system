pub use bincode::error::{DecodeError, EncodeError};
use bincode::{decode_from_std_read, encode_into_std_write};

mod errors;
pub mod unsafe_types;
pub mod config;

pub use errors::*;
use serde::{Deserialize, Serialize};

use crate::config::VirtualFsConfig;

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
  #[strum(to_string = "Finished patching target process")]
  FinishedPatching,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct EntryData {
  pub socket_name: String,
  pub fs_config: VirtualFsConfig,
}

impl EntryData {
  pub fn encode(self) -> Result<Vec<u8>, HookError> {
    Ok(serde_json::to_string(&self)?.into())
  }

  pub fn decode(data: &str) -> Result<Self, HookError> {
    Ok(serde_json::from_str(data)?)
  }
}
