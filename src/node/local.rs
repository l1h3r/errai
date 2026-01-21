#![expect(static_mut_refs, reason = "yep")]

use hashbrown::HashMap;
use std::sync::OnceLock;

use crate::bifs::TimerService;
use crate::consts;
use crate::core::Atom;
use crate::core::InternalPid;
use crate::core::ProcTable;
use crate::erts::System;
use crate::loom::sync::RwLock;
use crate::proc::ProcData;

// SAFETY: NODE uses static mut for shutdown ownership transfer.
//
// Initialization: get_or_init() provides thread-safe single initialization
// Normal operation: All access via immutable &'static references
// Shutdown: take() called once at coordinated shutdown to consume the node
//
// No concurrent mutable access occurs. Contained types (ProcTable, RwLock,
// TimerService) provide their own internal synchronization.
static mut NODE: OnceLock<LocalNode> = OnceLock::new();

/// Local node state.
///
/// Centralizes all runtime subsystems: process table, name registry, and
/// timer service, etc. Initialized at startup, consumed at shutdown.
pub(crate) struct LocalNode {
  /// Global process table mapping PIDs to process data.
  procs: ProcTable<ProcData>,
  /// Global name registry mapping registered names to PIDs.
  names: RwLock<HashMap<Atom, InternalPid>>,
  /// Timer service managing all process timers.
  timer: TimerService,
}

impl LocalNode {
  /// Creates a new local node with default capacity.
  #[inline]
  pub(crate) fn new() -> Self {
    let cpus: usize = System::available_cpus();

    Self {
      procs: ProcTable::with_capacity(consts::MAX_REGISTERED_PROCS),
      names: RwLock::new(HashMap::with_capacity(consts::CAP_REGISTERED_NAMES)),
      timer: TimerService::new(cpus),
    }
  }

  /// Shuts down the node, consuming all subsystems.
  ///
  /// Sends shutdown signals to all services and waits for cleanup.
  ///
  /// Safe to call multiple times (subsequent calls are no-ops).
  pub(crate) async fn shutdown() {
    // SAFETY: Shutdown called once at runtime exit
    let Some(node) = (unsafe { NODE.take() }) else {
      tracing::warn!("node already shut down");
      return;
    };

    node
      .timer
      .shutdown(consts::SHUTDOWN_TIMEOUT_WHEEL_WORKER)
      .await;
  }

  /// Returns a reference to the singleton node.
  ///
  /// Initializes the node on first call (thread-safe).
  #[inline]
  pub(crate) fn this() -> &'static Self {
    // SAFETY: get_or_init provides thread-safe initialization
    unsafe { NODE.get_or_init(Self::new) }
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

  /// Returns a reference to the timer service.
  #[inline]
  pub(crate) fn timer() -> &'static TimerService {
    &Self::this().timer
  }
}
