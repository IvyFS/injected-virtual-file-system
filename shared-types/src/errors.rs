use std::{error::Error, sync::PoisonError};

#[derive(Debug, thiserror::Error)]
pub enum HookError {
  #[error("could not find function: {function} in module: {module}")]
  FunctionNotFound { function: String, module: String },
  #[error("pointer to function: {function} in module: {module} was null")]
  FunctionPtrNull { function: String, module: String },
  #[error("Frida-Gum error: {cause}\ncontext: {context}")]
  GumError {
    context: String,
    cause: frida_gum::Error,
  },
  #[error("mutex error: {0}")]
  MutexError(#[source] Box<dyn Error>),
  #[error("encoding error: {0}")]
  BinEncodeError(#[from] bincode::error::EncodeError),
  #[error("decoding error: {0}")]
  BinDecodeError(#[from] bincode::error::DecodeError),
  #[error("json error: {0}")]
  JsonError(#[from] serde_json::error::Error),

  #[error("Failed to cast raw const ptr of type {typ}")]
  RawConstPtrCast { typ: String },
  #[error("Failed to cast raw mut ptr of type {typ}")]
  RawMutPtrCast { typ: String },

  #[cfg(windows)]
  #[error("Failed to allocate Rust string for conversion from UTF-16 string")]
  FromUtf16(#[from] std::string::FromUtf16Error),
  #[cfg(windows)]
  #[error("Failed to get path from file handle")]
  PathFromFileHandle,
}

impl<T: 'static> From<PoisonError<T>> for HookError {
  fn from(value: PoisonError<T>) -> Self {
    HookError::MutexError(Box::new(value))
  }
}

pub trait ErrorContext<T> {
  fn with_context(self, context: &str) -> Result<T, HookError>;
}

impl<T> ErrorContext<T> for Result<T, frida_gum::Error> {
  fn with_context(self, context: &str) -> Result<T, HookError> {
    match self {
      Ok(val) => Ok(val),
      Err(cause) => Err(HookError::GumError {
        context: context.to_owned(),
        cause,
      }),
    }
  }
}
