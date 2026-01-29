use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

/// Error returned when the process table has reached its capacity.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) struct ProcInsertError;

impl Display for ProcInsertError {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    f.write_str("too many processes")
  }
}

impl Error for ProcInsertError {}
