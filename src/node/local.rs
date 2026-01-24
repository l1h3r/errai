use hashbrown::HashMap;
use std::sync::OnceLock;

use crate::consts;
use crate::core::Atom;
use crate::core::InternalPid;
use crate::core::ProcTable;
use crate::loom::sync::RwLock;
use crate::proc::ProcData;

static NODE: OnceLock<LocalNode> = OnceLock::new();

/// Local node state.
///
/// Centralizes all runtime subsystems: process table, name registry, etc.
pub(crate) struct LocalNode {
  /// Global process table mapping PIDs to process data.
  procs: ProcTable<ProcData>,
  /// Global name registry mapping registered names to PIDs.
  names: RwLock<HashMap<Atom, InternalPid>>,
}

impl LocalNode {
  /// Creates a new local node with default capacity.
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      procs: ProcTable::with_capacity(consts::MAX_REGISTERED_PROCS),
      names: RwLock::new(HashMap::with_capacity(consts::CAP_REGISTERED_NAMES)),
    }
  }

  /// Returns a reference to the local node.
  #[inline]
  pub(crate) fn this() -> &'static Self {
    NODE.get_or_init(Self::new)
  }

  /// Returns a reference to the process table.
  #[inline]
  pub(crate) fn procs() -> &'static ProcTable<ProcData> {
    &Self::this().procs
  }

  /// Returns a reference to the name registry.
  #[inline]
  pub(crate) fn names() -> &'static RwLock<HashMap<Atom, InternalPid>> {
    &Self::this().names
  }
}
