use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::erts::ProcessId;
use crate::lang::Atom;
use crate::lang::ExternalPid;
use crate::lang::InternalPid;
use crate::lang::RawPid;

/// An internal/external process identifier.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum DynPid {
  Internal(InternalPid),
  External(ExternalPid),
}

impl DynPid {
  /// Returns the raw PID bits.
  #[inline]
  pub const fn bits(&self) -> RawPid {
    match self {
      Self::Internal(inner) => inner.bits(),
      Self::External(inner) => inner.bits(),
    }
  }

  /// Returns the name of the node that spawned this PID,
  /// or `None` if the PID is an internal PID.
  #[inline]
  pub const fn node(&self) -> Option<Atom> {
    match self {
      Self::Internal(_) => None,
      Self::External(inner) => Some(inner.node()),
    }
  }

  /// Returns `true` if the PID is internal (created on this node).
  #[inline]
  pub const fn is_internal(&self) -> bool {
    matches!(self, Self::Internal(_))
  }

  /// Returns `true` if the PID is external (created on another node).
  #[inline]
  pub const fn is_external(&self) -> bool {
    matches!(self, Self::External(_))
  }
}

impl Debug for DynPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Internal(inner) => Debug::fmt(inner, f),
      Self::External(inner) => Debug::fmt(inner, f),
    }
  }
}

impl Display for DynPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Internal(inner) => Display::fmt(inner, f),
      Self::External(inner) => Display::fmt(inner, f),
    }
  }
}

impl From<InternalPid> for DynPid {
  #[inline]
  fn from(other: InternalPid) -> Self {
    Self::Internal(other)
  }
}

impl From<ExternalPid> for DynPid {
  #[inline]
  fn from(other: ExternalPid) -> Self {
    Self::External(other)
  }
}

impl ProcessId for DynPid {
  #[inline]
  fn bits(&self) -> RawPid {
    self.bits()
  }

  #[inline]
  fn node(&self) -> Option<Atom> {
    self.node()
  }

  #[inline]
  fn into_dyn(self) -> DynPid {
    self
  }
}
