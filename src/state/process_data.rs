use hashbrown::HashMap;
use core::cell::UnsafeCell;
use core::num::NonZeroU64;
use core::ops::Deref;
use core::sync::atomic::Ordering;
use parking_lot::RwLock;
use std::sync::OnceLock;
use tokio::task::JoinHandle;
use triomphe::Arc;

// -----------------------------------------------------------------------------
// Proc Internal
// -----------------------------------------------------------------------------

/// Mutable process state.
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
  pub fn is_enabled(&self) -> bool {
    self.unlink.is_none()
  }

  /// Returns `true` if the link is disabled (unlink in progress).
  #[inline]
  pub fn is_disabled(&self) -> bool {
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
