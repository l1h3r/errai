use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::core::Atom;
use crate::core::Term;

/// The reason for stopping the execution of a running process.
#[derive(Clone)]
pub enum Exit {
  /// The reason for termination is an atom.
  ///
  /// Typically, this is used for internal, predefined reasons.
  Atom(Atom),
  /// The reason for termination is an arbitrary value.
  Term(Term),
}

impl Exit {
  /// The process exited normally.
  pub const NORMAL: Self = Self::Atom(Atom::NORMAL);

  /// The process was forcefully terminated.
  pub const KILLED: Self = Self::Atom(Atom::KILLED);

  /// The monitored process is not alive.
  pub const NOPROC: Self = Self::Atom(Atom::NOPROC);

  /// The monitored process resides on a disconnected node.
  pub const NOCONN: Self = Self::Atom(Atom::NOCONN);

  /// Returns `true` if the exit reason is [`NORMAL`][Atom::NORMAL].
  #[inline]
  pub fn is_normal(&self) -> bool {
    matches!(self, Self::Atom(atom) if *atom == Atom::NORMAL)
  }

  /// Returns `true` if the exit reason is [`KILLED`][Atom::KILLED].
  #[inline]
  pub fn is_killed(&self) -> bool {
    matches!(self, Self::Atom(atom) if *atom == Atom::KILLED)
  }

  /// Returns `true` if the exit reason is [`NOPROC`][Atom::NOPROC].
  #[inline]
  pub fn is_noproc(&self) -> bool {
    matches!(self, Self::Atom(atom) if *atom == Atom::NOPROC)
  }

  /// Returns `true` if the exit reason is [`NOCONN`][Atom::NOCONN].
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
