use hashbrown::HashMap;
use parking_lot::RwLock;
use std::cell::UnsafeCell;
use std::num::NonZeroU64;
use std::sync::OnceLock;
use tokio::task::JoinHandle;

use crate::core::Atom;
use crate::core::Exit;
use crate::core::LocalDest;
use crate::core::LocalPid;
use crate::core::MonitorRef;
use crate::erts::ProcDict;
use crate::erts::ProcFlags;
use crate::erts::ProcMail;
use crate::erts::ProcSend;
use crate::utils::atomic::AtomicNzU64;

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
#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcData {
  pub(crate) readonly: ProcReadOnly,
  pub(crate) internal: UnsafeCell<ProcInternal>,
  pub(crate) external: RwLock<ProcExternal>,
}

unsafe impl Send for ProcData {}
unsafe impl Sync for ProcData {}

// -----------------------------------------------------------------------------
// Proc Read-only
// -----------------------------------------------------------------------------

/// Immutable process data accessible without locking.
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
  pub(crate) mpid: LocalPid,

  /// Sending side of the process signal queue.
  pub(crate) send: ProcSend,

  /// Process that spawned this one.
  pub(crate) root: Option<LocalPid>,

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
  pub(crate) fn new(send: ProcSend, root: Option<LocalPid>) -> Self {
    Self {
      mpid: LocalPid::ROOT_PROC,
      send,
      root,
      task: OnceLock::new(),
      puid: AtomicNzU64::default(),
    }
  }
}

// -----------------------------------------------------------------------------
// Proc Internal
// -----------------------------------------------------------------------------

/// Mutable process state.
///
/// # Initialization
///
/// The `group_leader` field is initialized to an invalid value (0) and
/// must be updated during process setup.
#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcInternal {
  /// Process flags.
  pub(crate) flags: ProcFlags,

  /// Internal message queue.
  pub(crate) inbox: ProcMail,

  /// Linked process information.
  pub(crate) links: HashMap<LocalPid, ProcLink>,

  /// Monitors created by this process
  pub(crate) monitor_send: HashMap<MonitorRef, ProcMonitor>,

  /// Monitors watching this process.
  pub(crate) monitor_recv: HashMap<MonitorRef, ProcMonitor>,

  /// Process dictionary (key-value store).
  pub(crate) dictionary: ProcDict,

  /// I/O leader process.
  pub(crate) group_leader: LocalPid,
}

impl ProcInternal {
  /// Creates a new internal process data section.
  ///
  /// The `group_leader` field is initialized to an invalid value (0) and
  /// must be updated during process setup.
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      flags: ProcFlags::empty(),
      inbox: ProcMail::new(),
      links: HashMap::new(),
      monitor_send: HashMap::new(),
      monitor_recv: HashMap::new(),
      dictionary: ProcDict::new(),
      group_leader: LocalPid::ROOT_PROC,
    }
  }
}

// -----------------------------------------------------------------------------
// Proc External
// -----------------------------------------------------------------------------

/// Rarely-modified process state.
///
/// This section contains data that is set once or rarely modified during
/// process lifetime.
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

// -----------------------------------------------------------------------------
// Proc Link
// -----------------------------------------------------------------------------

/// State of a process link.
///
/// Process links are bidirectional connections that propagate exit signals.
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct ProcLink {
  unlink: Option<NonZeroU64>,
}

impl ProcLink {
  /// Creates a new enabled link.
  #[inline]
  pub(crate) fn new() -> Self {
    Self { unlink: None }
  }

  /// Returns `true` if the link is enabled (not being unlinked).
  #[inline]
  pub(crate) fn is_enabled(&self) -> bool {
    self.unlink.is_none()
  }

  /// Returns `true` if the link is disabled (unlink in progress).
  #[inline]
  pub(crate) fn is_disabled(&self) -> bool {
    !self.is_enabled()
  }

  /// Enables the link (clears unlink state).
  #[inline]
  pub(crate) fn enable(&mut self) {
    self.unlink = None;
  }

  /// Disables the link with the given unlink ID.
  #[inline]
  pub(crate) fn disable(&mut self, unlink: NonZeroU64) {
    self.unlink = Some(unlink);
  }

  /// Returns `true` if the stored unlink ID matches `ulid`.
  #[inline]
  pub(crate) fn matches(&self, ulid: NonZeroU64) -> bool {
    self.unlink.map(|id| id == ulid).unwrap_or(false)
  }
}

// -----------------------------------------------------------------------------
// Proc Monitor
// -----------------------------------------------------------------------------

/// State of a process monitor.
///
/// Monitors are unidirectional: one process (origin) watches another (target).
/// When the target terminates, a DOWN message is sent to the origin.
#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcMonitor {
  origin: LocalPid,
  target: LocalDest,
}

impl ProcMonitor {
  /// Creates a new monitor state.
  #[inline]
  pub(crate) const fn new(origin: LocalPid, target: LocalDest) -> Self {
    Self { origin, target }
  }

  /// Returns the PID of the process that created the monitor.
  #[inline]
  pub(crate) fn origin(&self) -> LocalPid {
    self.origin
  }

  /// Returns the destination being monitored.
  #[inline]
  pub(crate) fn target(&self) -> LocalDest {
    self.target
  }
}
