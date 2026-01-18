use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::core::Atom;
use crate::core::InternalPid;
use crate::core::ProcessId;

/// Identifier uniquely naming a process on a remote node.
///
/// External PIDs combine an [`InternalPid`] with a node name, enabling
/// process addressing across distributed nodes.
///
/// # Format
///
/// External PIDs display as `#PID<Node.Number.Serial>` where `Node` is the
/// atom slot identifying the originating node.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct ExternalPid {
  bits: InternalPid,
  node: Atom,
}

impl ExternalPid {
  /// Creates a new external PID from internal bits and node name.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::{Atom, ExternalPid, InternalPid};
  ///
  /// let bits = InternalPid::from_bits(0x456);
  /// let node = Atom::new("remote@host");
  /// let pid = ExternalPid::new(bits, node);
  /// ```
  #[inline]
  pub const fn new(bits: InternalPid, node: Atom) -> Self {
    Self { bits, node }
  }

  /// Returns the internal PID component.
  ///
  /// This extracts the local PID portion, discarding node information.
  #[inline]
  pub const fn bits(&self) -> InternalPid {
    self.bits
  }

  /// Returns the node name that spawned this process.
  ///
  /// This identifies which node in the distributed system owns the process.
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
