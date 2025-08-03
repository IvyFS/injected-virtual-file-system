use serde::{Deserialize, Serialize};

use crate::HookError;

pub use postcard::accumulator::{CobsAccumulator, FeedResult};

#[derive(Debug, PartialEq, strum::Display, Serialize, Deserialize)]
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
  pub fn send(self, writer: &mut impl std::io::Write) -> Result<(), HookError> {
    Ok(writer.write_all(&postcard::to_allocvec_cobs(&self)?)?)
  }
}
