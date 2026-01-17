use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::core::Atom;
use crate::core::InternalRef;

/// An external reference.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct ExternalRef {
  bits: InternalRef,
  node: Atom,
}

impl ExternalRef {
  /// Creates a new `ExternalRef`.
  #[inline]
  pub const fn new(bits: InternalRef, node: Atom) -> Self {
    Self { bits, node }
  }

  /// Returns the raw reference bits.
  #[inline]
  pub const fn bits(&self) -> InternalRef {
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
    let node: u32 = self.node.into_slot();
    let bits: [u32; 3] = self.bits.into_bits();

    write!(f, "#Ref<{}.{}.{}.{}>", node, bits[2], bits[1], bits[0])
  }
}

impl From<(InternalRef, Atom)> for ExternalRef {
  #[inline]
  fn from(other: (InternalRef, Atom)) -> Self {
    Self::new(other.0, other.1)
  }
}
