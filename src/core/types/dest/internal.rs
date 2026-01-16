use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::core::Atom;
use crate::core::InternalPid;

/// Represents an internal message destination.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum InternalDest {
  /// A process identifier.
  Proc(InternalPid),
  /// A registered name.
  Name(Atom),
}

impl InternalDest {
  /// Returns `true` if the destination is a PID.
  #[inline]
  pub const fn is_proc(&self) -> bool {
    matches!(self, Self::Proc(_))
  }

  /// Returns `true` if the destination is a registered name.
  #[inline]
  pub const fn is_name(&self) -> bool {
    matches!(self, Self::Name(_))
  }
}

impl Debug for InternalDest {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Proc(inner) => Debug::fmt(inner, f),
      Self::Name(inner) => Debug::fmt(inner, f),
    }
  }
}

impl Display for InternalDest {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Proc(inner) => Display::fmt(inner, f),
      Self::Name(inner) => Display::fmt(inner, f),
    }
  }
}

impl From<InternalPid> for InternalDest {
  #[inline]
  fn from(other: InternalPid) -> Self {
    Self::Proc(other)
  }
}

impl From<Atom> for InternalDest {
  #[inline]
  fn from(other: Atom) -> Self {
    Self::Name(other)
  }
}

impl From<&'static str> for InternalDest {
  #[inline]
  fn from(other: &'static str) -> Self {
    Self::Name(Atom::new(other))
  }
}
