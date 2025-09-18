#[cfg(windows)]
use std::ffi::OsString;
use std::{error::Error, path::PathBuf, sync::PoisonError};

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
  #[error("postcard error: {0}")]
  Postcard(#[from] postcard::Error),
  #[error("json error: {0}")]
  JsonError(#[from] serde_json::error::Error),

  #[error("Failed to cast raw const ptr of type {typ} to {} reference", unsafe_ptr_cast_ref_type(*.mutable_ref))]
  RawConstPtrCast { typ: String, mutable_ref: bool },
  #[error("Failed to cast raw mut ptr of type {typ} to {} reference", unsafe_ptr_cast_ref_type(*.mutable_ref))]
  RawMutPtrCast { typ: String, mutable_ref: bool },

  #[cfg(windows)]
  #[error("Failed to allocate Rust string for conversion from UTF-16 string")]
  FromUtf16(#[from] std::string::FromUtf16Error),
  #[cfg(windows)]
  #[error("Failed to get path from file handle")]
  PathFromFileHandle,
  #[cfg(windows)]
  #[error("Failed to init UNICODE_STRING from {source_str:?}:\nNTSTATUS: {nt_status:#?}\ncontains_nul: {contains_nul:#?}", nt_status = .nt_status.map(|e| format!("{e:x}")))]
  UnicodeInit {
    source_str: OsString,
    nt_status: Option<i32>,
    contains_nul: Option<widestring::error::ContainsNul<u16>>,
  },
  #[cfg(windows)]
  #[error("windows-core error: {0:?}")]
  WindowsCore(#[source] Box<dyn Error>),

  #[error("failed to canonicalize path: {path} with err: {cause}")]
  Canonicalize {
    path: PathBuf,
    cause: std::io::Error,
  },

  #[error("no virtual path")]
  NoVirtualPath,

  #[error("std::io::Error {0}")]
  StdIo(#[from] std::io::Error),
  #[error("error: {source}\nadditional context: {context}")]
  WithContext {
    #[source]
    source: Box<dyn Error>,
    context: String,
  },
  #[error("Other: {0}")]
  Other(String),
}

impl<T: 'static> From<PoisonError<T>> for HookError {
  fn from(value: PoisonError<T>) -> Self {
    HookError::MutexError(Box::new(value))
  }
}

pub trait ErrorContext<T> {
  fn with_context<M: ToString>(self, context: impl Fn() -> M) -> Result<T, HookError>;
}

impl<T> ErrorContext<T> for Result<T, frida_gum::Error> {
  fn with_context<M: ToString>(self, context: impl Fn() -> M) -> Result<T, HookError> {
    match self {
      Ok(val) => Ok(val),
      Err(cause) => Err(HookError::GumError {
        context: context().to_string(),
        cause,
      }),
    }
  }
}

impl<T, E: Into<HookError>> ErrorContext<T> for Result<T, E> {
  fn with_context<M: ToString>(self, context: impl Fn() -> M) -> Result<T, HookError> {
    match self {
      Ok(val) => Ok(val),
      Err(err) => Err(HookError::WithContext {
        source: Box::new(err.into()),
        context: context().to_string(),
      }),
    }
  }
}

fn unsafe_ptr_cast_ref_type(mutable: bool) -> &'static str {
  if mutable { "mutable" } else { "immutable" }
}
