use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::lang::Term;

/// The reason used to stop the execution of a running process.
#[derive(Clone)]
pub enum ExitReason {
  /// A graceful process exit.
  Normal,
  /// A forceful process exit.
  Kill,
  /// A process exit due to a specific reason.
  Term(Term),
}

impl ExitReason {
  /// Returns `true` if the exit reason is `Normal`.
  #[inline]
  pub const fn is_normal(&self) -> bool {
    matches!(self, Self::Normal)
  }

  /// Returns `true` if the exit reason is `Kill`.
  #[inline]
  pub const fn is_kill(&self) -> bool {
    matches!(self, Self::Kill)
  }

  /// Returns `true` if the exit reason is `Term`.
  #[inline]
  pub const fn is_term(&self) -> bool {
    matches!(self, Self::Term(_))
  }
}

impl Debug for ExitReason {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Normal => f.write_str("Normal"),
      Self::Kill => f.write_str("Kill"),
      Self::Term(term) => Debug::fmt(term, f),
    }
  }
}

impl Display for ExitReason {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Normal => f.write_str("Normal"),
      Self::Kill => f.write_str("Kill"),
      Self::Term(term) => Display::fmt(term, f),
    }
  }
}

impl From<Term> for ExitReason {
  #[inline]
  fn from(other: Term) -> Self {
    Self::Term(other)
  }
}
