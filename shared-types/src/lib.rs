use std::time::Duration;

pub mod config;
mod errors;
pub mod message;
pub mod unsafe_types;

pub use errors::*;
pub use message::Message;

pub const DEFAULT_HEARTBEAT: Duration = Duration::from_millis(100);
