use hashbrown::HashMap;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::num::NonZeroU64;
use std::sync::OnceLock;
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
use crate::loom::sync::atomic::Ordering;
use crate::proc::ProcDict;
use crate::proc::ProcLink;
use crate::proc::ProcMail;
use crate::proc::ProcMonitor;
use crate::proc::ProcSend;
use crate::tyre::num::AtomicNzU64;

// -----------------------------------------------------------------------------
// Proc Data
// -----------------------------------------------------------------------------

/// Top-level process data container with three locking domains.
///
/// This structure organizes process state into sections with different
/// access patterns and locking requirements:
///
/// 1. **Read-only**: No lock needed, contains immutable data
/// 2. **Internal**: Task-local UnsafeCell, zero-overhead access via guard
/// 3. **External**: RwLock-protected, contains rarely-modified state
///
/// The internal state uses `UnsafeCell` instead of locks because it's
/// only accessed by the owning process task. Access is mediated by
/// `TaskGuard` which proves we're in the correct task context.
///
/// # Drop Behavior
///
/// Logs process termination for debugging. The actual cleanup (removing
/// from process table) happens in [`ProcTask::drop`].
///
/// [`ProcTask::drop`]: crate::proc::ProcTask
#[derive(Debug)]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
pub(crate) struct ProcData {
  pub(crate) readonly: ProcReadOnly,
  pub(crate) internal: UnsafeCell<ProcInternal>,
  pub(crate) external: RwLock<ProcExternal>,
}

// SAFETY: ProcData is Send because we can transfer ownership between threads.
unsafe impl Send for ProcData {}

// SAFETY: ProcData is Sync because while Arc<ProcData> is accessible from
// multiple threads, only `readonly` and `external` are accessed concurrently.
// The `internal` field is only accessed from the owning task via TaskGuard.
unsafe impl Sync for ProcData {}

// -----------------------------------------------------------------------------
// Proc Read-only
// -----------------------------------------------------------------------------

/// Immutable process data accessible without locking.
///
/// This section contains data that is set once at process creation and
/// never modified, enabling lock-free reads from any thread.
///
/// # Fields
///
/// - `mpid`: Process identifier (set after table insertion)
/// - `send`: Signal sender for this process (cloneable, shareable)
/// - `root`: Spawning process PID (if spawned by another process)
/// - `task`: Tokio task handle (set after spawn)
/// - `puid`: Atomic counter for generating unique IDs within this process
///
/// # Initialization
///
/// The `mpid` field is initially set to an invalid value (0) and updated
/// after successful insertion into the process table. The `task` field is
/// set once the process future is spawned onto Tokio.
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
  /// Process-unique identifier counter.
  pub(crate) puid: AtomicNzU64,
}

impl ProcReadOnly {
  /// Creates a new read-only process data section.
  ///
  /// The `mpid` field is initialized to an invalid value (0) and must be
  /// updated after process table insertion.
  #[inline]
  pub(crate) fn new(send: ProcSend, root: Option<InternalPid>) -> Self {
    Self {
      mpid: InternalPid::from_bits(0),
      send,
      root,
      task: OnceLock::new(),
      puid: AtomicNzU64::default(),
    }
  }

  /// Returns the next process-unique identifier.
  ///
  /// This counter is used for generating unique IDs within the process
  /// context, such as unlink identifiers.
  #[inline]
  pub(crate) fn next_puid(&self) -> NonZeroU64 {
    self.puid.fetch_add(1, Ordering::Relaxed)
  }

  // ---------------------------------------------------------------------------
  // Signals
  // ---------------------------------------------------------------------------

  /// Sends a message signal to this process.
  #[inline]
  pub(crate) fn send_message<T>(&self, from: InternalPid, data: T)
  where
    T: Into<DynMessage>,
  {
    SignalSend::new(from, data.into()).emit(self);
  }

  /// Sends an exit signal to this process.
  #[inline]
  pub(crate) fn send_exit(&self, from: InternalPid, exit: Exit) {
    SignalExit::new(from, exit).emit(self);
  }

  /// Sends a link signal to this process.
  #[inline]
  pub(crate) fn send_link(&self, from: InternalPid) {
    SignalLink::new(from).emit(self);
  }

  /// Sends a link-exit signal to this process.
  #[inline]
  pub(crate) fn send_link_exit(&self, from: InternalPid, exit: Exit) {
    SignalLinkExit::new(from, exit).emit(self);
  }

  /// Sends an unlink signal to this process.
  #[inline]
  pub(crate) fn send_unlink(&self, from: InternalPid, ulid: NonZeroU64) {
    SignalUnlink::new(from, ulid).emit(self);
  }

  /// Sends an unlink acknowledgment signal to this process.
  #[inline]
  pub(crate) fn send_unlink_ack(&self, from: InternalPid, ulid: NonZeroU64) {
    SignalUnlinkAck::new(from, ulid).emit(self);
  }

  /// Sends a monitor signal to this process.
  #[inline]
  pub(crate) fn send_monitor<T>(&self, from: InternalPid, mref: MonitorRef, item: T)
  where
    T: Into<ExternalDest>,
  {
    SignalMonitor::new(from, mref, item.into()).emit(self);
  }

  /// Sends a monitor-down signal to this process.
  #[inline]
  pub(crate) fn send_monitor_down(&self, from: InternalPid, mref: MonitorRef, info: Exit) {
    SignalMonitorDown::new(from, mref, info).emit(self);
  }

  /// Sends a demonitor signal to this process.
  #[inline]
  pub(crate) fn send_demonitor(&self, from: InternalPid, mref: MonitorRef) {
    SignalDemonitor::new(from, mref).emit(self);
  }
}

// -----------------------------------------------------------------------------
// Proc Internal
// -----------------------------------------------------------------------------

/// Mutable process state.
///
/// This section contains frequently-modified process state that requires
/// exclusive access.
///
/// # Fields
///
/// - `flags`: Process behavior flags (trap_exit, async_dist, etc.)
/// - `inbox`: Internal message queue for selective receive
/// - `links`: Active process links indexed by PID
/// - `monitor_send`: Monitors created by this process
/// - `monitor_recv`: Monitors watching this process
/// - `dictionary`: Process dictionary (key-value store)
/// - `group_leader`: I/O leader process for this process
///
/// # Initialization
///
/// The `group_leader` field is initialized to an invalid value (0) and
/// must be updated during process setup.
#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcInternal {
  /// Process flags.
  pub(crate) flags: ProcessFlags,
  /// Internal message queue.
  pub(crate) inbox: ProcMail,
  /// Linked process information.
  pub(crate) links: HashMap<InternalPid, ProcLink>,
  /// Monitors requested by this process.
  pub(crate) monitor_send: HashMap<MonitorRef, ProcMonitor>,
  /// Monitors watching this process.
  pub(crate) monitor_recv: HashMap<MonitorRef, ProcMonitor>,
  /// Process dictionary (key-value store).
  pub(crate) dictionary: ProcDict,
  /// I/O leader process.
  pub(crate) group_leader: InternalPid,
}

impl ProcInternal {
  /// Creates a new internal process data section.
  ///
  /// The `group_leader` field is initialized to an invalid value (0) and
  /// must be updated during process setup.
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

  /// Appends a message to the end of the process inbox. The caller must
  /// ensure signal-ordering is preserved.
  #[inline]
  pub(crate) fn send<M>(&mut self, message: M)
  where
    M: Into<DynMessage>,
  {
    self.inbox.push(message.into());
  }
}

// -----------------------------------------------------------------------------
// Proc External
// -----------------------------------------------------------------------------

/// Rarely-modified process state.
///
/// This section contains data that is set once or rarely modified during
/// process lifetime.
///
/// # Fields
///
/// - `name`: Registered name (if process is registered)
/// - `exit`: Exit reason (set once when process terminates)
#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcExternal {
  /// Registered name (if any).
  pub(crate) name: Option<Atom>,
  /// Exit reason (set once on termination).
  pub(crate) exit: OnceLock<Exit>,
}

impl ProcExternal {
  /// Creates a new external process data section.
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      name: None,
      exit: OnceLock::new(),
    }
  }
}
