use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::lang::Atom;
use crate::lang::ExternalPid;
use crate::lang::InternalPid;

/// Represents an internal or external message destination.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternalDest {
  /// An internal process identifier.
  InternalProc(InternalPid),
  /// An external process identifier.
  ExternalProc(ExternalPid),
  /// An internal registered name.
  InternalName(Atom),
  /// An external registered name/node pair.
  ExternalName(Atom, Atom),
}

impl ExternalDest {
  /// Returns `true` if the destination is a PID.
  #[inline]
  pub const fn is_proc(&self) -> bool {
    matches!(self, Self::InternalProc(_) | Self::ExternalProc(_))
  }

  /// Returns `true` if the destination is an internal PID.
  #[inline]
  pub const fn is_internal_proc(&self) -> bool {
    matches!(self, Self::InternalProc(_))
  }

  /// Returns `true` if the destination is an external PID.
  #[inline]
  pub const fn is_external_proc(&self) -> bool {
    matches!(self, Self::ExternalProc(_))
  }

  /// Returns `true` if the destination is a registered name.
  #[inline]
  pub const fn is_name(&self) -> bool {
    matches!(self, Self::InternalName(_) | Self::ExternalName(_, _))
  }

  /// Returns `true` if the destination is an internal registered name.
  #[inline]
  pub const fn is_internal_name(&self) -> bool {
    matches!(self, Self::InternalName(_))
  }

  /// Returns `true` if the destination is an external registered name.
  #[inline]
  pub const fn is_external_name(&self) -> bool {
    matches!(self, Self::ExternalName(_, _))
  }
}

impl Debug for ExternalDest {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::InternalProc(inner) => Debug::fmt(inner, f),
      Self::ExternalProc(inner) => Debug::fmt(inner, f),
      Self::InternalName(inner) => Debug::fmt(inner, f),
      Self::ExternalName(inner, _node) => Debug::fmt(inner, f),
    }
  }
}

impl Display for ExternalDest {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::InternalProc(inner) => Display::fmt(inner, f),
      Self::ExternalProc(inner) => Display::fmt(inner, f),
      Self::InternalName(inner) => Display::fmt(inner, f),
      Self::ExternalName(inner, _node) => Display::fmt(inner, f),
    }
  }
}

impl From<InternalPid> for ExternalDest {
  #[inline]
  fn from(other: InternalPid) -> Self {
    Self::InternalProc(other)
  }
}

impl From<ExternalPid> for ExternalDest {
  #[inline]
  fn from(other: ExternalPid) -> Self {
    Self::ExternalProc(other)
  }
}

impl From<Atom> for ExternalDest {
  #[inline]
  fn from(other: Atom) -> Self {
    Self::InternalName(other)
  }
}

impl From<&'static str> for ExternalDest {
  #[inline]
  fn from(other: &'static str) -> Self {
    Self::InternalName(Atom::new(other))
  }
}

impl From<(Atom, Atom)> for ExternalDest {
  #[inline]
  fn from(other: (Atom, Atom)) -> Self {
    Self::ExternalName(other.0, other.1)
  }
}

impl From<(&'static str, &'static str)> for ExternalDest {
  #[inline]
  fn from(other: (&'static str, &'static str)) -> Self {
    Self::ExternalName(Atom::new(other.0), Atom::new(other.1))
  }
}
