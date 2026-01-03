use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::erts::ProcessId;
use crate::lang::Atom;
use crate::lang::DynPid;
use crate::lang::RawPid;

/// An internal process identifier.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct InternalPid {
  bits: RawPid,
}

impl InternalPid {
  /// Creates a new `InternalPid`.
  #[inline]
  pub const fn new(bits: RawPid) -> Self {
    Self { bits }
  }

  /// Returns the raw PID bits.
  #[inline]
  pub const fn bits(&self) -> RawPid {
    self.bits
  }
}

impl Debug for InternalPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Debug::fmt(&self.bits, f)
  }
}

impl Display for InternalPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(&self.bits, f)
  }
}

impl From<RawPid> for InternalPid {
  #[inline]
  fn from(other: RawPid) -> Self {
    Self::new(other)
  }
}

impl ProcessId for InternalPid {
  #[inline]
  fn bits(&self) -> RawPid {
    self.bits()
  }

  #[inline]
  fn node(&self) -> Option<Atom> {
    None
  }

  #[inline]
  fn into_dyn(self) -> DynPid {
    DynPid::Internal(self)
  }
}
