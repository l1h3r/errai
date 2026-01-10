use parking_lot::RwLock;
use std::ops::Deref;
use tokio::task::JoinHandle;
use triomphe::Arc;

use crate::bifs;
use crate::erts::ProcessDict;
use crate::erts::ProcessFlags;
use crate::erts::ProcessSend;
use crate::erts::SignalQueue;
use crate::lang::Atom;
use crate::lang::InternalPid;

// -----------------------------------------------------------------------------
// @type - ProcessRoot
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcessRoot {
  pub(crate) mpid: InternalPid,
  pub(crate) send: ProcessSend,
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
  pub(crate) signal_queue: SignalQueue,
}

// -----------------------------------------------------------------------------
// @type - ProcessSlot
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
pub(crate) struct ProcessSlot {
  pub(crate) root: ProcessRoot,
  pub(crate) data: RwLock<ProcessData>,
  pub(crate) dict: ProcessDict,
}

// -----------------------------------------------------------------------------
// @type - ProcessTask
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct ProcessTask {
  pub(crate) slot: Arc<ProcessSlot>,
}

impl Drop for ProcessTask {
  fn drop(&mut self) {
    if bifs::process_delete(self.root.mpid).is_none() {
      eprintln!("[errai]: Dangling Process ({})", self.root.mpid);
    }
  }
}

impl Deref for ProcessTask {
  type Target = ProcessSlot;

  #[inline]
  fn deref(&self) -> &Self::Target {
    &*self.slot
  }
}
