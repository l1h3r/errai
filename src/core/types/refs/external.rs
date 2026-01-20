use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::core::Atom;
use crate::core::InternalRef;

/// Reference identifying a remote object on a distributed node.
///
/// External references combine an [`InternalRef`] with a node identifier,
/// enabling unique identification across multiple nodes in a distributed
/// system.
///
/// # Format
///
/// External references display as `#Ref<N.X.Y.Z>` where:
///
/// - `N`: Node atom slot (identifies which node created the reference)
/// - `X`, `Y`, `Z`: 32-bit components from the internal reference
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct ExternalRef {
  bits: InternalRef,
  node: Atom,
}

impl ExternalRef {
  /// Creates a new external reference from internal bits and node name.
  #[inline]
  pub const fn new(bits: InternalRef, node: Atom) -> Self {
    Self { bits, node }
  }

  /// Returns the internal reference component.
  ///
  /// This extracts the local reference portion, discarding node information.
  #[inline]
  pub const fn bits(&self) -> InternalRef {
    self.bits
  }

  /// Returns the node name that created this reference.
  ///
  /// This identifies which node in the distributed system originated the
  /// reference.
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use crate::core::Atom;
  use crate::core::ExternalRef;
  use crate::core::InternalRef;

  const IREF: InternalRef = InternalRef::from_bits([1, 2, 3]);
  const NODE: Atom = Atom::UNDEFINED;

  #[test]
  fn test_new() {
    let eref: ExternalRef = ExternalRef::new(IREF, NODE);

    assert_eq!(eref.bits(), IREF);
    assert_eq!(eref.node(), NODE);
  }

  #[test]
  fn test_clone() {
    let src: ExternalRef = ExternalRef::new(IREF, NODE);
    let dst: ExternalRef = src.clone();

    assert_eq!(src, dst);
  }

  #[test]
  fn test_copy() {
    let src: ExternalRef = ExternalRef::new(IREF, NODE);
    let dst: ExternalRef = src;

    assert_eq!(src, dst);
  }

  #[test]
  fn test_display() {
    let src: ExternalRef = ExternalRef::new(IREF, NODE);
    let fmt: String = format!("{src}");

    assert_eq!(fmt, "#Ref<6.3.2.1>");
  }

  #[test]
  fn test_debug_equals_display() {
    let src: ExternalRef = ExternalRef::new(IREF, NODE);
    let fmt: String = format!("{src}");

    assert_eq!(fmt, format!("{src:?}"));
  }
}
