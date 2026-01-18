use std::backtrace::Backtrace;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::error::ExceptionClass;
use crate::error::ExceptionGroup;

/// A structured exception with class, group, message, and backtrace.
///
/// Exceptions are raised via the [`raise!`] macro and propagated through
/// panic unwinding.
///
/// # Display Format
///
/// Exceptions format as: `{class}:{group} - {message}`
///
/// Example: `error:badarg - value must be non-negative`
///
/// [`raise!`]: crate::raise
pub struct Exception {
  class: ExceptionClass,
  group: ExceptionGroup,
  error: String,
  trace: Backtrace,
}

impl Exception {
  /// Creates a new exception with the given class, group, and message.
  ///
  /// Automatically captures a backtrace at the call site for debugging.
  /// This function is typically invoked via the [`raise!`] macro rather
  /// than directly.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::error::{Exception, ExceptionClass, ExceptionGroup};
  ///
  /// let exception = Exception::new(
  ///   ExceptionClass::Error,
  ///   ExceptionGroup::BadArg,
  ///   "invalid input",
  /// );
  /// ```
  ///
  /// [`raise!`]: crate::raise
  #[inline]
  pub fn new<T>(class: ExceptionClass, group: ExceptionGroup, error: T) -> Self
  where
    T: Display,
  {
    Self {
      class,
      group,
      error: error.to_string(),
      trace: Backtrace::capture(),
    }
  }

  /// Returns the exception's severity class.
  #[inline]
  pub const fn class(&self) -> ExceptionClass {
    self.class
  }

  /// Returns the exception's error category.
  #[inline]
  pub const fn group(&self) -> ExceptionGroup {
    self.group
  }

  /// Returns the human-readable error message.
  #[inline]
  pub const fn error(&self) -> &str {
    self.error.as_str()
  }

  /// Returns the captured backtrace.
  ///
  /// Backtrace availability depends on the `RUST_BACKTRACE` environment
  /// variable and platform support.
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
    write!(f, "{}:{} - {}", self.class, self.group, self.error)
  }
}

impl Error for Exception {}
