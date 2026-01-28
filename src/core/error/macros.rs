//! Internal error handling macros.
//!
//! Provides two categories of error handling:
//!
//! - [`fatal!`]: For unrecoverable runtime bugs (invariant violations)
//! - [`raise!`]: For recoverable system errors (capacity limits)

/// Displays a system error message and aborts the program.
///
/// Use this for unrecoverable errors that indicate bugs in the runtime
/// implementation itself. The program prints a diagnostic message and
/// immediately aborts without unwinding.
///
/// # Examples
///
/// ```ignore
/// if slot.is_null() {
///   fatal!("null slot in active process table");
/// }
/// ```
macro_rules! fatal {
  ($error:expr) => {{
    ::std::eprintln!(
      "{}:{}: (SysInv) a system invariant has been broken: {}",
      ::std::file!(),
      ::std::line!(),
      $error,
    );

    ::std::process::abort();
  }};
}

/// Panics with a recoverable system error.
///
/// Use this for resource exhaustion that may be recoverable at a higher
/// level (e.g., catching panics, supervision trees). The program panics
/// with a diagnostic message indicating which limit was exceeded.
///
/// # Examples
///
/// ```ignore
/// if atoms.len() >= MAX_ATOM_COUNT {
///   raise!(Error, SysCap, "too many atoms");
/// }
/// ```
macro_rules! raise {
  (Error, SysCap, $error:expr) => {
    ::std::panic!(
      "{}:{}: (SysCap) a system limit has been reached: {}",
      ::std::file!(),
      ::std::line!(),
      $error,
    )
  };
}

pub(crate) use fatal;
pub(crate) use raise;
