use std::backtrace::Backtrace;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

// -----------------------------------------------------------------------------
// raise!
// -----------------------------------------------------------------------------

macro_rules! raise {
  ($class:ident, $group:ident, $error:literal $(,)?) => {
    ::std::panic!(
      "{}",
      $crate::core::Exception::new(
        $crate::core::ExceptionClass::$class,
        $crate::core::ExceptionGroup::$group,
        $error,
      ),
    );
  };
}

pub(crate) use raise;

// -----------------------------------------------------------------------------
// Exception Class
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExceptionClass {
  Error,
}

// -----------------------------------------------------------------------------
// Exception Group
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExceptionGroup {
  BadArg,
  SysCap,
}

impl ExceptionGroup {
  #[inline]
  const fn label(&self) -> &'static str {
    match self {
      Self::BadArg => "(BadArg) errors were found with the given argument(s)",
      Self::SysCap => "(SysCap) a system limit has been reached",
    }
  }
}

// -----------------------------------------------------------------------------
// Exception
// -----------------------------------------------------------------------------

/// Error type returned from invalid runtime operations.
pub struct Exception {
  class: ExceptionClass,
  group: ExceptionGroup,
  error: &'static str,
  trace: Backtrace,
}

impl Exception {
  /// Creates a new `Exception`.
  #[inline]
  pub(crate) fn new(class: ExceptionClass, group: ExceptionGroup, error: &'static str) -> Self {
    Self {
      class,
      group,
      error,
      trace: Backtrace::capture(),
    }
  }

  /// Returns the exception class.
  #[inline]
  pub const fn class(&self) -> ExceptionClass {
    self.class
  }

  /// Returns the exception group.
  #[inline]
  pub const fn group(&self) -> ExceptionGroup {
    self.group
  }

  /// Returns the exception error message.
  #[inline]
  pub const fn error(&self) -> &'static str {
    self.error
  }

  /// Returns the thread stack backtrace leading up to the exception.
  #[inline]
  pub const fn trace(&self) -> &Backtrace {
    &self.trace
  }
}

impl Debug for Exception {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(self, f)
  }
}

impl Display for Exception {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    write!(f, "[errai]: {}: {}", self.group.label(), self.error)
  }
}

impl Error for Exception {}
