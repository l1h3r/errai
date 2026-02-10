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
/// # Standard Exit Reasons
///
/// The runtime defines several well-known exit reasons:
///
/// - [`Exit::NORMAL`]: Clean shutdown with no errors
/// - [`Exit::KILLED`]: Forceful termination (unconditional)
/// - [`Exit::NOPROC`]: Monitored process doesn't exist
/// - [`Exit::NOCONN`]: Remote node connection lost
///
/// # Custom Exit Reasons
///
/// Applications can use custom exit reasons via the [`Exit::Term`] variant,
/// allowing structured error information to propagate through supervision trees.
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
#[derive(Clone, PartialEq)]
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
  /// This is the expected exit reason for processes that complete their work
  /// successfully.
  pub const NORMAL: Self = Self::Atom(Atom::NORMAL);

  /// Exit reason indicating forced process termination.
  ///
  /// This reason is used when a process must be terminated immediately.
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
  fn test_display_custom() {
    assert_eq!(format!("{}", Exit::Atom(Atom::new("timeout"))), "timeout");
  }

  #[test]
  fn test_debug_custom() {
    assert_eq!(format!("{:?}", Exit::Atom(Atom::new("timeout"))), "timeout");
  }

  #[test]
  fn test_from_atom() {
    let atom: Atom = Atom::new("shutdown");
    let exit: Exit = Exit::from(atom);

    assert_eq!(exit, Exit::Atom(atom));
  }

  #[test]
  fn test_from_well_known_atom() {
    assert!(Exit::from(Atom::NORMAL).is_normal());
  }

  #[test]
  fn test_from_term_integer() {
    let term: Term = Term::new(123_u32);
    let exit: Exit = Exit::from(term.clone());

    assert_eq!(exit, Exit::Term(term));
  }

  #[test]
  fn test_from_term_string() {
    let term: Term = Term::new("error message");
    let exit: Exit = Exit::from(term.clone());

    assert_eq!(exit, Exit::Term(term));
  }
}
