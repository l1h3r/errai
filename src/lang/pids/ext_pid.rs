use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::erts::ProcessId;
use crate::lang::Atom;
use crate::lang::DynPid;
use crate::lang::RawPid;

/// An external process identifier.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct ExternalPid {
  bits: RawPid,
  node: Atom,
}

impl ExternalPid {
  /// Creates a new `ExternalPid`.
  #[inline]
  pub const fn new(bits: RawPid, node: Atom) -> Self {
    Self { bits, node }
  }

  /// Returns the raw PID bits.
  #[inline]
  pub const fn bits(&self) -> RawPid {
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
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    write!(
      f,
      "#PID<{}.{}.{}>",
      self.node,
      self.bits.number(),
      self.bits.serial()
    )
  }
}

impl From<(RawPid, Atom)> for ExternalPid {
  #[inline]
  fn from(other: (RawPid, Atom)) -> Self {
    Self::new(other.0, other.1)
  }
}

impl ProcessId for ExternalPid {
  #[inline]
  fn bits(&self) -> RawPid {
    self.bits()
  }

  #[inline]
  fn node(&self) -> Option<Atom> {
    Some(self.node)
  }

  #[inline]
  fn into_dyn(self) -> DynPid {
    DynPid::External(self)
  }
}
