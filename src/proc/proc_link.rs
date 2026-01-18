use std::num::NonZeroU64;

use crate::core::ExternalDest;
use crate::core::InternalPid;

// -----------------------------------------------------------------------------
// Proc Link
// -----------------------------------------------------------------------------

/// State of a process link with unlinking support.
///
/// Process links are bidirectional connections that propagate exit signals.
/// Unlinking is asynchronous and requires tracking to avoid race conditions.
///
/// # States
///
/// - **Enabled** (`unlink == None`): Link is active
/// - **Disabled** (`unlink == Some(id)`): Unlink initiated with ID `id`
///
/// # Unlink Protocol
///
/// 1. Sender calls `unlink(pid)`, generates unique ID, disables link
/// 2. Sender sends UNLINK signal with ID to target
/// 3. Target processes UNLINK, sends back UNLINK_ACK with same ID
/// 4. Sender receives UNLINK_ACK, removes link if ID matches
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
  ///
  /// This marks the link as "unlink in progress" and stores the ID for
  /// later verification when the UNLINK_ACK arrives.
  #[inline]
  pub(crate) fn disable(&mut self, unlink: NonZeroU64) {
    self.unlink = Some(unlink);
  }

  /// Returns `true` if the stored unlink ID matches `ulid`.
  ///
  /// Used when processing UNLINK_ACK signals to verify the ID.
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
///
/// # Fields
///
/// - `origin`: Process that created the monitor
/// - `target`: Process or name being monitored
///
/// # Monitor Types
///
/// Monitors can watch:
/// - Local processes (by PID)
/// - Remote processes (by PID)
/// - Registered names (local or remote)
#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcMonitor {
  origin: InternalPid,
  target: ExternalDest,
}

impl ProcMonitor {
  /// Creates a new monitor state.
  #[inline]
  pub(crate) const fn new(origin: InternalPid, target: ExternalDest) -> Self {
    Self { origin, target }
  }

  /// Returns the monitoring process PID.
  #[inline]
  pub(crate) fn origin(&self) -> InternalPid {
    self.origin
  }

  /// Returns the monitored destination.
  #[inline]
  pub(crate) fn target(&self) -> ExternalDest {
    self.target
  }
}
