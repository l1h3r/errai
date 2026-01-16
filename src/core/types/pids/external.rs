use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::core::Atom;
use crate::core::InternalPid;
use crate::core::ProcessId;

/// An external process identifier.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct ExternalPid {
  bits: InternalPid,
  node: Atom,
}

impl ExternalPid {
  /// Creates a new `ExternalPid`.
  #[inline]
  pub const fn new(bits: InternalPid, node: Atom) -> Self {
    Self { bits, node }
  }

  /// Returns the raw PID bits.
  #[inline]
  pub const fn bits(&self) -> InternalPid {
    self.bits
  }

  /// Returns the name of the node that spawned this PID.
  #[inline]
  pub const fn node(&self) -> Atom {
    self.node
  }
}

impl Debug for ExternalPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(self, f)
  }
}

impl Display for ExternalPid {
  fn fmt(&self, _f: &mut Formatter<'_>) -> Result {
    todo!()
  }
}

impl From<(InternalPid, Atom)> for ExternalPid {
  #[inline]
  fn from(other: (InternalPid, Atom)) -> Self {
    Self::new(other.0, other.1)
  }
}

impl ProcessId for ExternalPid {
  const DISTRIBUTED: bool = true;

  #[inline]
  fn into_internal(self) -> InternalPid {
    self.bits
  }

  #[inline]
  fn into_external(self) -> Option<ExternalPid> {
    Some(self)
  }
}
