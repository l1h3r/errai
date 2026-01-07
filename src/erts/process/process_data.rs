use parking_lot::RwLock;
use std::ops::Deref;
use tokio::task::JoinHandle;
use triomphe::Arc;

use crate::erts::ProcessDict;
use crate::erts::ProcessFlags;
use crate::lang::Atom;
use crate::lang::InternalPid;

// -----------------------------------------------------------------------------
// @type - ProcessRoot
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcessRoot {
  pub(crate) id: InternalPid,
}

// -----------------------------------------------------------------------------
// @type - ProcessData
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcessData {
  // ---------------------------------------------------------------------------
  // Frequently accessed fields
  // ---------------------------------------------------------------------------
  pub(crate) pflags: ProcessFlags,
  // ---------------------------------------------------------------------------
  // Infrequently accessed fields
  // ---------------------------------------------------------------------------
  pub(crate) task: Option<JoinHandle<()>>,
  pub(crate) name: Option<Atom>,
  pub(crate) spawn_parent: Option<InternalPid>,
  pub(crate) group_leader: InternalPid,
}

// -----------------------------------------------------------------------------
// @type - ProcessSlot
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcessSlot {
  pub(crate) root: ProcessRoot,
  pub(crate) data: RwLock<ProcessData>,
  pub(crate) dict: ProcessDict,
}

// -----------------------------------------------------------------------------
// @type - ProcessTask
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcessTask {
  pub(crate) slot: Arc<ProcessSlot>,
}

impl Deref for ProcessTask {
  type Target = ProcessSlot;

  #[inline]
  fn deref(&self) -> &Self::Target {
    &*self.slot
  }
}
