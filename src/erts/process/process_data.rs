use parking_lot::RwLock;
use std::ops::Deref;
use tokio::task::JoinHandle;
use triomphe::Arc;

use crate::bifs;
use crate::erts::DynMessage;
use crate::erts::ProcessDict;
use crate::erts::ProcessFlags;
use crate::erts::ProcessSend;
use crate::lang::Atom;
use crate::lang::ExitReason;
use crate::lang::InternalPid;

// -----------------------------------------------------------------------------
// @type - ProcessData
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcessData {
  /// Process flags.
  pub(crate) flags: ProcessFlags,
  /// Reason for process termination.
  pub(crate) exit: Option<ExitReason>,
  /// Registered name.
  pub(crate) name: Option<Atom>,
  /// Handle to the internal process task.
  pub(crate) task: Option<JoinHandle<()>>,
  /// Process I/O leader.
  pub(crate) group_leader: InternalPid,
  /// Internal message queue.
  pub(crate) inbox_buffer: Vec<DynMessage>,
}

// -----------------------------------------------------------------------------
// @type - ProcessSlot
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
pub(crate) struct ProcessSlot {
  /// PID of the process.
  pub(crate) mpid: InternalPid,
  /// Sending side of the process signal queue.
  pub(crate) send: ProcessSend,
  /// Process that spawned this one.
  pub(crate) root: Option<InternalPid>,
  /// Protected process state.
  pub(crate) data: RwLock<ProcessData>,
  /// The process dictionary.
  pub(crate) dict: ProcessDict,
}

impl Drop for ProcessSlot {
  fn drop(&mut self) {
    tracing::event!(
      target: "errai",
      tracing::Level::TRACE,
      ?self.mpid,
      ?self.send,
      ?self.root,
      ?self.data,
      ?self.dict,
      "Process drop",
    );
  }
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
    if bifs::process_delete(self.mpid).is_none() {
      tracing::warn!(pid = %self.mpid, "Dangling process");
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
