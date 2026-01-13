use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::lang::Atom;
use crate::lang::InternalRef;

/// An external reference.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct ExternalRef {
  iref: InternalRef,
  node: Atom,
}

impl ExternalRef {
  /// Creates a new `ExternalRef`.
  #[inline]
  pub const fn new(iref: InternalRef, node: Atom) -> Self {
    Self { iref, node }
  }

  /// Returns the raw reference iref.
  #[inline]
  pub const fn iref(&self) -> InternalRef {
    self.iref
  }

  /// Returns the name of the node that spawned this reference.
  #[inline]
  pub const fn node(&self) -> Atom {
    self.node
  }
}

impl Debug for ExternalRef {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(self, f)
  }
}

impl Display for ExternalRef {
  fn fmt(&self, _f: &mut Formatter<'_>) -> Result {
    todo!()
  }
}

impl From<(InternalRef, Atom)> for ExternalRef {
  #[inline]
  fn from(other: (InternalRef, Atom)) -> Self {
    Self::new(other.0, other.1)
  }
}
