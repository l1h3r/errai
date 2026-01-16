use hashbrown::HashMap;
use parking_lot::Mutex;
use parking_lot::RwLock;
use std::num::NonZeroU64;
use std::sync::OnceLock;
use std::sync::atomic::Ordering;
use tokio::task::JoinHandle;

use crate::core::Atom;
use crate::core::Exit;
use crate::core::ExternalDest;
use crate::core::InternalPid;
use crate::core::MonitorRef;
use crate::erts::DynMessage;
use crate::erts::ProcessFlags;
use crate::erts::SignalDemonitor;
use crate::erts::SignalEmit;
use crate::erts::SignalExit;
use crate::erts::SignalLink;
use crate::erts::SignalLinkExit;
use crate::erts::SignalMonitor;
use crate::erts::SignalMonitorDown;
use crate::erts::SignalSend;
use crate::erts::SignalUnlink;
use crate::erts::SignalUnlinkAck;
use crate::proc::ProcDict;
use crate::proc::ProcLink;
use crate::proc::ProcMail;
use crate::proc::ProcMonitor;
use crate::proc::ProcSend;
use crate::tyre::num::AtomicNzU64;

// -----------------------------------------------------------------------------
// Proc Data
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
pub(crate) struct ProcData {
  pub(crate) readonly: ProcReadOnly,
  pub(crate) internal: Mutex<ProcInternal>,
  pub(crate) external: RwLock<ProcExternal>,
}

impl Drop for ProcData {
  fn drop(&mut self) {
    tracing::trace!(pid = %self.readonly.mpid, "Proc Drop");
  }
}

// -----------------------------------------------------------------------------
// Proc Read-only
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcReadOnly {
  /// PID of the process.
  pub(crate) mpid: InternalPid,
  /// Sending side of the process signal queue.
  pub(crate) send: ProcSend,
  /// Process that spawned this one.
  pub(crate) root: Option<InternalPid>,
  /// Handle to the internal process task.
  pub(crate) task: OnceLock<JoinHandle<()>>,
  /// Process-unique identifier.
  pub(crate) puid: AtomicNzU64,
}

impl ProcReadOnly {
  /// Create a new `ProcReadOnly`.
  ///
  /// Note: `mpid` is set to invalid value.
  #[inline]
  pub(crate) fn new(send: ProcSend, root: Option<InternalPid>) -> Self {
    Self {
      mpid: InternalPid::from_bits(0),
      send,
      root,
      task: OnceLock::new(),
      puid: AtomicNzU64::new(),
    }
  }

  /// Returns the next process unique identifier.
  #[inline]
  pub(crate) fn next_puid(&self) -> NonZeroU64 {
    self.puid.fetch_add(1, Ordering::Relaxed)
  }

  // ---------------------------------------------------------------------------
  // Signals
  // ---------------------------------------------------------------------------

  #[inline]
  pub(crate) fn send_message<T>(&self, from: InternalPid, data: T)
  where
    T: Into<DynMessage>,
  {
    SignalSend::new(from, data.into()).emit(self);
  }

  #[inline]
  pub(crate) fn send_exit(&self, from: InternalPid, exit: Exit) {
    SignalExit::new(from, exit).emit(self);
  }

  #[inline]
  pub(crate) fn send_link(&self, from: InternalPid) {
    SignalLink::new(from).emit(self);
  }

  #[inline]
  pub(crate) fn send_link_exit(&self, from: InternalPid, exit: Exit) {
    SignalLinkExit::new(from, exit).emit(self);
  }

  #[inline]
  pub(crate) fn send_unlink(&self, from: InternalPid, ulid: NonZeroU64) {
    SignalUnlink::new(from, ulid).emit(self);
  }

  #[inline]
  pub(crate) fn send_unlink_ack(&self, from: InternalPid, ulid: NonZeroU64) {
    SignalUnlinkAck::new(from, ulid).emit(self);
  }

  #[inline]
  pub(crate) fn send_monitor<T>(&self, from: InternalPid, mref: MonitorRef, item: T)
  where
    T: Into<ExternalDest>,
  {
    SignalMonitor::new(from, mref, item.into()).emit(self);
  }

  #[inline]
  pub(crate) fn send_monitor_down(&self, from: InternalPid, mref: MonitorRef, info: Exit) {
    SignalMonitorDown::new(from, mref, info).emit(self);
  }

  #[inline]
  pub(crate) fn send_demonitor(&self, from: InternalPid, mref: MonitorRef) {
    SignalDemonitor::new(from, mref).emit(self);
  }
}

// -----------------------------------------------------------------------------
// Proc Internal
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcInternal {
  /// Process flags.
  pub(crate) flags: ProcessFlags,
  /// Internal message queue.
  pub(crate) inbox: ProcMail,
  /// Linked process information.
  pub(crate) links: HashMap<InternalPid, ProcLink>,
  /// The state of monitors requested by the process.
  pub(crate) monitor_send: HashMap<MonitorRef, ProcMonitor>,
  /// The state of monitors received by the process.
  pub(crate) monitor_recv: HashMap<MonitorRef, ProcMonitor>,
  /// The process dictionary.
  pub(crate) dictionary: ProcDict,
  /// Process I/O leader.
  pub(crate) group_leader: InternalPid,
}

impl ProcInternal {
  /// Create a new `ProcInternal`.
  ///
  /// Note: `group_leader` is set to invalid value.
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      flags: ProcessFlags::empty(),
      inbox: ProcMail::new(),
      links: HashMap::new(),
      monitor_send: HashMap::new(),
      monitor_recv: HashMap::new(),
      dictionary: ProcDict::new(),
      group_leader: InternalPid::from_bits(0),
    }
  }
}

// -----------------------------------------------------------------------------
// Proc External
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcExternal {
  /// Registered name.
  pub(crate) name: Option<Atom>,
  /// Reason for termination.
  pub(crate) exit: OnceLock<Exit>,
}

impl ProcExternal {
  /// Create a new `ProcExternal`.
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      name: None,
      exit: OnceLock::new(),
    }
  }
}
