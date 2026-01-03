use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::lang::RawRef;

/// An internal reference.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct InternalRef {
  bits: RawRef,
}

impl InternalRef {
  /// Creates a new `InternalRef`.
  #[inline]
  pub const fn new(bits: RawRef) -> Self {
    Self { bits }
  }

  /// Returns the raw reference bits.
  #[inline]
  pub const fn bits(&self) -> RawRef {
    self.bits
  }
}

impl Debug for InternalRef {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Debug::fmt(&self.bits, f)
  }
}

impl Display for InternalRef {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(&self.bits, f)
  }
}

impl From<RawRef> for InternalRef {
  #[inline]
  fn from(other: RawRef) -> Self {
    Self::new(other)
  }
}
