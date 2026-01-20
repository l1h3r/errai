use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

/// Exception severity classification.
///
/// Currently, only error-level exceptions are supported. Future versions
/// may add other classes for different handling semantics.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExceptionClass {
  /// Fatal error requiring process termination or supervision.
  Error,
}

impl Display for ExceptionClass {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Error => f.write_str("error"),
    }
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use crate::error::ExceptionClass;

  #[test]
  fn test_display() {
    assert_eq!(format!("{}", ExceptionClass::Error), "error");
  }
}
