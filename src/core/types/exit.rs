//! Process exit reasons used for termination, linking, and monitoring semantics.
//!
//! This module provides the [`Exit`] type, which represents why a process
//! terminated. Exit reasons propagate through process links and monitors,
//! enabling fault detection and supervision hierarchies.
//!
//! # Exit Propagation
//!
//! When a process terminates, its exit reason is sent to:
//!
//! - **Linked processes**: Receive exit signals that may cause termination
//! - **Monitoring processes**: Receive down messages with the exit reason
//!
//! The behavior depends on process flags like `trap_exit`.
//!
//! # Standard Exit Reasons
//!
//! The runtime defines several well-known exit reasons:
//!
//! - [`Exit::NORMAL`]: Clean shutdown with no errors
//! - [`Exit::KILLED`]: Forceful termination (unconditional)
//! - [`Exit::NOPROC`]: Monitored process doesn't exist
//! - [`Exit::NOCONN`]: Remote node connection lost
//!
//! # Custom Exit Reasons
//!
//! Applications can use custom exit reasons via the [`Exit::Term`] variant,
//! allowing structured error information to propagate through supervision trees.
//!
//! # Examples
//!
//! ```
//! use errai::core::{Atom, Exit, Term};
//!
//! // Standard exit reasons
//! let normal = Exit::NORMAL;
//! let killed = Exit::KILLED;
//!
//! // Custom atom-based reason
//! let shutdown = Exit::from(Atom::new("shutdown"));
//!
//! // Custom structured reason
//! let error = Exit::from(Term::new("database connection failed"));
//!
//! // Check exit type
//! assert!(normal.is_normal());
//! assert!(!killed.is_normal());
//! ```

use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::core::Atom;
use crate::core::Term;

/// Reason describing why a process stopped executing.
///
/// Exit reasons serve two primary purposes:
///
/// 1. **Diagnostic**: Explain what caused process termination
/// 2. **Propagation**: Determine how linked/monitored processes react
///
/// # Variants
///
/// - [`Exit::Atom`]: Predefined or well-known exit reasons
/// - [`Exit::Term`]: Custom or structured exit reasons
///
/// # Normal vs Abnormal Termination
///
/// The exit reason [`Exit::NORMAL`] has special semantics: it doesn't
/// cause linked processes to terminate (unless they're trapping exits).
/// All other exit reasons are considered abnormal and propagate through
/// process links.
///
/// # Examples
///
/// ```
/// use errai::core::{Atom, Exit};
///
/// let exit = Exit::NORMAL;
///
/// if exit.is_normal() {
///   println!("Process terminated cleanly");
/// } else {
///   println!("Process crashed: {}", exit);
/// }
/// ```
#[derive(Clone)]
pub enum Exit {
  /// Exit reason represented by a predefined atom.
  ///
  /// This variant is used for standard runtime reasons like [`Exit::NORMAL`],
  /// [`Exit::KILLED`], and application-defined symbolic reasons.
  Atom(Atom),
  /// Exit reason represented by an arbitrary runtime value.
  ///
  /// This variant allows user-defined or structured termination reasons,
  /// enabling rich error context to flow through supervision trees.
  Term(Term),
}

impl Exit {
  /// Exit reason indicating normal process termination.
  ///
  /// Normal exits don't cause linked processes to terminate (unless they
  /// have `trap_exit` enabled). This is the expected exit reason for
  /// processes that complete their work successfully.
  pub const NORMAL: Self = Self::Atom(Atom::NORMAL);

  /// Exit reason indicating forced process termination.
  ///
  /// Killed exits propagate unconditionally through process links and
  /// cannot be trapped. This reason is used when a process must be
  /// terminated immediately.
  pub const KILLED: Self = Self::Atom(Atom::KILLED);

  /// Exit reason indicating a nonexistent monitored process.
  ///
  /// This reason appears in down messages when monitoring a process that
  /// doesn't exist or has already terminated.
  pub const NOPROC: Self = Self::Atom(Atom::NOPROC);

  /// Exit reason indicating a disconnected remote node.
  ///
  /// This reason appears when a monitored remote process becomes unreachable
  /// due to network partition or node shutdown.
  pub const NOCONN: Self = Self::Atom(Atom::NOCONN);

  /// Returns `true` if this exit reason represents normal termination.
  ///
  /// Only [`Exit::NORMAL`] is considered a normal exit. All other reasons,
  /// including custom atoms, are treated as abnormal.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::{Atom, Exit};
  ///
  /// assert!(Exit::NORMAL.is_normal());
  /// assert!(!Exit::KILLED.is_normal());
  /// assert!(!Exit::from(Atom::new("custom")).is_normal());
  /// ```
  #[inline]
  pub fn is_normal(&self) -> bool {
    matches!(self, Self::Atom(atom) if *atom == Atom::NORMAL)
  }

  /// Returns `true` if this exit reason represents forced termination.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Exit;
  ///
  /// assert!(Exit::KILLED.is_killed());
  /// assert!(!Exit::NORMAL.is_killed());
  /// ```
  #[inline]
  pub fn is_killed(&self) -> bool {
    matches!(self, Self::Atom(atom) if *atom == Atom::KILLED)
  }

  /// Returns `true` if this exit reason represents a missing process.
  ///
  /// This typically appears in monitor down messages when the target
  /// process never existed or was already dead when monitoring began.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Exit;
  ///
  /// assert!(Exit::NOPROC.is_noproc());
  /// assert!(!Exit::NORMAL.is_noproc());
  /// ```
  #[inline]
  pub fn is_noproc(&self) -> bool {
    matches!(self, Self::Atom(atom) if *atom == Atom::NOPROC)
  }

  /// Returns `true` if this exit reason represents a disconnected node.
  ///
  /// This typically appears when a monitored process on a remote node
  /// becomes unreachable due to network issues or node failure.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Exit;
  ///
  /// assert!(Exit::NOCONN.is_noconn());
  /// assert!(!Exit::NORMAL.is_noconn());
  /// ```
  #[inline]
  pub fn is_noconn(&self) -> bool {
    matches!(self, Self::Atom(atom) if *atom == Atom::NOCONN)
  }
}

impl Debug for Exit {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Atom(inner) => Debug::fmt(inner, f),
      Self::Term(inner) => Debug::fmt(inner, f),
    }
  }
}

impl Display for Exit {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Atom(inner) => Display::fmt(inner, f),
      Self::Term(inner) => Display::fmt(inner, f),
    }
  }
}

impl From<Atom> for Exit {
  #[inline]
  fn from(other: Atom) -> Self {
    Self::Atom(other)
  }
}

impl From<Term> for Exit {
  #[inline]
  fn from(other: Term) -> Self {
    Self::Term(other)
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use crate::core::Atom;
  use crate::core::Exit;
  use crate::core::Term;

  #[test]
  fn test_constants_exist() {
    let _unused: Exit = Exit::NORMAL;
    let _unused: Exit = Exit::KILLED;
    let _unused: Exit = Exit::NOPROC;
    let _unused: Exit = Exit::NOCONN;
  }

  #[test]
  fn test_normal_is_atom() {
    assert!(
      matches!(Exit::NORMAL, Exit::Atom(_)),
      "NORMAL should be Atom variant"
    );
  }

  #[test]
  fn test_killed_is_atom() {
    assert!(
      matches!(Exit::KILLED, Exit::Atom(_)),
      "KILLED should be Atom variant"
    );
  }

  #[test]
  fn test_noproc_is_atom() {
    assert!(
      matches!(Exit::NOPROC, Exit::Atom(_)),
      "NOPROC should be Atom variant"
    );
  }

  #[test]
  fn test_noconnis_atom() {
    assert!(
      matches!(Exit::NOCONN, Exit::Atom(_)),
      "NOCONN should be Atom variant"
    );
  }

  #[test]
  fn test_is_normal() {
    assert!(Exit::NORMAL.is_normal());
    assert!(!Exit::KILLED.is_normal());
    assert!(!Exit::NOPROC.is_normal());
    assert!(!Exit::NOCONN.is_normal());
  }

  #[test]
  fn test_is_killed() {
    assert!(Exit::KILLED.is_killed());
    assert!(!Exit::NORMAL.is_killed());
    assert!(!Exit::NOPROC.is_killed());
    assert!(!Exit::NOCONN.is_killed());
  }

  #[test]
  fn test_is_noproc() {
    assert!(Exit::NOPROC.is_noproc());
    assert!(!Exit::NORMAL.is_noproc());
    assert!(!Exit::KILLED.is_noproc());
    assert!(!Exit::NOCONN.is_noproc());
  }

  #[test]
  fn test_is_noconn() {
    assert!(Exit::NOCONN.is_noconn());
    assert!(!Exit::NORMAL.is_noconn());
    assert!(!Exit::KILLED.is_noconn());
    assert!(!Exit::NOPROC.is_noconn());
  }

  #[test]
  fn test_is_normal_with_custom_atom() {
    let exit: Exit = Exit::from(Atom::new("custom"));

    assert!(!exit.is_normal());
  }

  #[test]
  fn test_term_variant_predicates() {
    let exit: Exit = Exit::from(Term::new("error"));

    assert!(!exit.is_normal());
    assert!(!exit.is_killed());
    assert!(!exit.is_noproc());
    assert!(!exit.is_noconn());
  }

  #[test]
  fn test_from_atom() {
    let atom: Atom = Atom::new("shutdown");
    let exit: Exit = Exit::from(atom);

    if let Exit::Atom(item) = exit {
      assert_eq!(item, atom);
    } else {
      panic!("Should be Atom variant");
    }
  }

  #[test]
  fn test_from_well_known_atom() {
    assert!(Exit::from(Atom::NORMAL).is_normal());
  }

  #[test]
  fn test_from_term_integer() {
    let term: Term = Term::new(123_u32);
    let exit: Exit = Exit::from(term);

    assert!(matches!(exit, Exit::Term(_)), "Should be Term variant");
  }

  #[test]
  fn test_from_term_string() {
    let term: Term = Term::new(String::from("error message"));
    let exit: Exit = Exit::from(term);

    assert!(matches!(exit, Exit::Term(_)), "Should be Term variant");
  }

  #[test]
  fn test_clone() {
    let src: Exit = Exit::NORMAL;
    let dst: Exit = src.clone();

    assert!(src.is_normal());
    assert!(dst.is_normal());
  }

  #[test]
  fn test_display() {
    assert_eq!(format!("{}", Exit::NORMAL), "normal");
    assert_eq!(format!("{}", Exit::KILLED), "killed");
    assert_eq!(format!("{}", Exit::NOPROC), "noproc");
    assert_eq!(format!("{}", Exit::NOCONN), "noconn");
  }

  #[test]
  fn test_debug() {
    assert_eq!(format!("{:?}", Exit::NORMAL), "normal");
    assert_eq!(format!("{:?}", Exit::KILLED), "killed");
    assert_eq!(format!("{:?}", Exit::NOPROC), "noproc");
    assert_eq!(format!("{:?}", Exit::NOCONN), "noconn");
  }

  #[test]
  fn test_custom_display() {
    let src: Exit = Exit::Atom(Atom::new("timeout"));
    let fmt: String = format!("{src}");

    assert_eq!(fmt, "timeout");
  }
}
