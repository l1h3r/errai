use crate::lang::Atom;
use crate::lang::DynPid;
use crate::lang::RawPid;

pub trait ProcessId {
  /// Returns the raw PID bits.
  fn bits(&self) -> RawPid;

  /// Returns the name of the node that spawned this PID,
  /// or `None` if the PID is an internal PID.
  fn node(&self) -> Option<Atom>;

  /// Converts `self` into a dynamic PID.
  fn into_dyn(self) -> DynPid;

  /// Returns `true` if the PID is internal (created on this node).
  #[inline]
  fn is_internal(&self) -> bool {
    self.node().is_none()
  }

  /// Returns `true` if the PID is external (created on another node).
  #[inline]
  fn is_external(&self) -> bool {
    self.node().is_some()
  }
}
