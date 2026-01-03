use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::lang::Atom;
use crate::lang::ExternalRef;
use crate::lang::InternalRef;
use crate::lang::RawRef;

/// An internal/external reference.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum DynRef {
  Internal(InternalRef),
  External(ExternalRef),
}

impl DynRef {
  /// Returns the raw reference bits.
  #[inline]
  pub const fn bits(&self) -> RawRef {
    match self {
      Self::Internal(inner) => inner.bits(),
      Self::External(inner) => inner.bits(),
    }
  }

  /// Returns the name of the node that spawned this reference,
  /// or `None` if the reference is an internal reference.
  #[inline]
  pub const fn node(&self) -> Option<Atom> {
    match self {
      Self::Internal(_) => None,
      Self::External(inner) => Some(inner.node()),
    }
  }

  /// Returns `true` if the reference is internal (created on this node).
  #[inline]
  pub const fn is_internal(&self) -> bool {
    matches!(self, Self::Internal(_))
  }

  /// Returns `true` if the reference is external (created on another node).
  #[inline]
  pub const fn is_external(&self) -> bool {
    matches!(self, Self::External(_))
  }
}

impl Debug for DynRef {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Internal(inner) => Debug::fmt(inner, f),
      Self::External(inner) => Debug::fmt(inner, f),
    }
  }
}

impl Display for DynRef {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Internal(inner) => Display::fmt(inner, f),
      Self::External(inner) => Display::fmt(inner, f),
    }
  }
}

impl From<InternalRef> for DynRef {
  #[inline]
  fn from(other: InternalRef) -> Self {
    Self::Internal(other)
  }
}

impl From<ExternalRef> for DynRef {
  #[inline]
  fn from(other: ExternalRef) -> Self {
    Self::External(other)
  }
}
