use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

/// Exception category indicating the nature of the error.
///
/// Groups provide semantic information for error handling and logging.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExceptionGroup {
  /// Invalid function argument or parameter.
  ///
  /// Indicates the caller provided data that violates function preconditions.
  BadArg,
  /// System capacity limit exceeded.
  ///
  /// Indicates resource exhaustion such as full process or atom tables.
  SysCap,
  /// Invalid system operation or state.
  ///
  /// Indicates an operation that cannot be performed in the current system state.
  SysInv,
}

impl ExceptionGroup {
  #[inline]
  pub(crate) const fn label(&self) -> &'static str {
    match self {
      Self::BadArg => "badarg",
      Self::SysCap => "syscap",
      Self::SysInv => "sysinv",
    }
  }
}

impl Display for ExceptionGroup {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::BadArg => f.write_str("(BadArg) errors were found with the given argument(s)"),
      Self::SysCap => f.write_str("(SysCap) a system limit has been reached"),
      Self::SysInv => f.write_str("(SysInv) a system invariant has been broken"),
    }
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use crate::error::ExceptionGroup;

  #[test]
  fn test_display() {
    let badarg: String = format!("{}", ExceptionGroup::BadArg);
    let syscap: String = format!("{}", ExceptionGroup::SysCap);
    let sysinv: String = format!("{}", ExceptionGroup::SysInv);

    assert!(badarg.starts_with("(BadArg)"));
    assert!(syscap.starts_with("(SysCap)"));
    assert!(sysinv.starts_with("(SysInv)"));
  }
}
