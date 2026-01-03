use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::lang::Atom;
use crate::lang::RawRef;

/// An external reference.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct ExternalRef {
  bits: RawRef,
  node: Atom,
}

impl ExternalRef {
  /// Creates a new `ExternalRef`.
  #[inline]
  pub const fn new(bits: RawRef, node: Atom) -> Self {
    Self { bits, node }
  }

  /// Returns the raw reference bits.
  #[inline]
  pub const fn bits(&self) -> RawRef {
    self.bits
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
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    todo!()
  }
}

impl From<(RawRef, Atom)> for ExternalRef {
  #[inline]
  fn from(other: (RawRef, Atom)) -> Self {
    Self::new(other.0, other.1)
  }
}
