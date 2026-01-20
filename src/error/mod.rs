//! Exception handling and error types for the Errai runtime.
//!
//! This module provides a panic-based exception system for the runtime.
//! Exceptions are categorized by class and group to enable structured
//! error handling patterns.
//!
//! # Exception Model
//!
//! Errai uses panics to propagate errors within process boundaries,
//! following the "let it crash" philosophy where process supervisors
//! handle recovery rather than local error handling.
//!
//! Exceptions carry three pieces of information:
//!
//! 1. **Class**: The severity level ([`Error`])
//! 2. **Group**: The error category ([`BadArg`], [`SysCap`], [`SysInv`])
//! 3. **Description**: A human-readable error message
//!
//! # Raising Exceptions
//!
//! Use the [`raise!`] macro to construct and panic with an exception:
//!
//! ```
//! use errai::raise;
//!
//! fn validate_input(value: i32) {
//!   if value < 0 {
//!     raise!(Error, BadArg, "value must be non-negative");
//!   }
//! }
//! ```
//!
//! The macro automatically captures a backtrace and formats the exception
//! message for panic handling.
//!
//! # Exception Groups
//!
//! - [`BadArg`]: Invalid function arguments
//! - [`SysCap`]: System capacity exhausted (table full, etc.)
//! - [`SysInv`]: Invalid system state or operation
//!
//! [`Error`]: ExceptionClass::Error
//! [`BadArg`]: ExceptionGroup::BadArg
//! [`SysCap`]: ExceptionGroup::SysCap
//! [`SysInv`]: ExceptionGroup::SysInv
//!
//! [`raise!`]: crate::raise!

mod exception;
mod exception_class;
mod exception_group;

pub use self::exception::Exception;
pub use self::exception_class::ExceptionClass;
pub use self::exception_group::ExceptionGroup;

// -----------------------------------------------------------------------------
// raise!
// -----------------------------------------------------------------------------

/// Raises an exception with the specified class, group, and message.
///
/// This macro constructs an [`Exception`] and immediately panics, allowing
/// process supervisors to handle the error.
///
/// # Examples
///
/// ```
/// # use errai::raise;
/// fn register_name(name: &str) {
///   if name.is_empty() {
///     raise!(Error, BadArg, "name cannot be empty");
///   }
/// }
/// ```
#[macro_export]
macro_rules! raise {
  ($class:ident, $group:ident, $error:expr $(,)?) => {
    ::std::panic!(
      "{}",
      $crate::error::Exception::new(
        $crate::error::ExceptionClass::$class,
        $crate::error::ExceptionGroup::$group,
        $error,
      ),
    )
  };
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::panic;

  #[test]
  fn test_raise_macro_badarg() {
    assert!(panic::catch_unwind(|| raise!(Error, BadArg, "test message")).is_err());
  }

  #[test]
  fn test_raise_macro_syscap() {
    assert!(panic::catch_unwind(|| raise!(Error, SysCap, "table full")).is_err());
  }

  #[test]
  fn test_raise_macro_sysinv() {
    assert!(panic::catch_unwind(|| raise!(Error, SysInv, "invalid state")).is_err());
  }
}
