use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

/// Error returned when attempting to access an invalid process.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) struct ProcAccessError;

impl Display for ProcAccessError {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    f.write_str("invalid process access")
  }
}

impl Error for ProcAccessError {}
