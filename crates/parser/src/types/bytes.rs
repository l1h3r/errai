use alloc::borrow::Borrow;
use core::fmt::Debug;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;
use core::ops::Deref;

use crate::types::Slice;

// =============================================================================
// Bytes
// =============================================================================

/// An owned slice of bytes.
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Bytes {
  inner: Box<[u8]>,
}

impl Bytes {
  pub(crate) const fn new(inner: Box<[u8]>) -> Self {
    Self { inner }
  }

  /// Returns a shared reference to the slice of bytes.
  #[inline]
  pub const fn as_slice(&self) -> &Slice {
    Slice::new(&self.inner)
  }
}

impl Debug for Bytes {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("Bytes { .. }")
  }
}

impl Deref for Bytes {
  type Target = Slice;

  #[inline]
  fn deref(&self) -> &Self::Target {
    self.as_slice()
  }
}

impl Borrow<Slice> for Bytes {
  #[inline]
  fn borrow(&self) -> &Slice {
    self.as_slice()
  }
}
