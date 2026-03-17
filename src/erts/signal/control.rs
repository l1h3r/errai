use std::num::NonZeroU64;

use crate::core::Exit;
use crate::core::LocalDest;
use crate::core::LocalPid;
use crate::core::MonitorRef;

// -----------------------------------------------------------------------------
// Control Signal
// -----------------------------------------------------------------------------

/// Control signals for process coordination and lifecycle.
///
/// Control signals manage:
///
/// - Process termination (Exit)
/// - Process links (Link, LinkExit, Unlink, UnlinkAck)
/// - Process monitors (Monitor, MonitorDown, Demonitor)
#[derive(Clone, Debug)]
pub(crate) enum ControlSignal {
  // ---------------------------------------------------------------------------
  // Termination Signals
  // ---------------------------------------------------------------------------
  /// Unconditional exit signal.
  Exit(SignalExit),
  // ---------------------------------------------------------------------------
  // Link Signals
  // ---------------------------------------------------------------------------
  /// Establish a link between processes.
  Link(SignalLink),
  /// Exit signal from a linked process.
  LinkExit(SignalLinkExit),
  /// Request to unlink from a process.
  Unlink(SignalUnlink),
  /// Acknowledgment of unlink request.
  UnlinkAck(SignalUnlinkAck),
  // ---------------------------------------------------------------------------
  // Monitor Signals
  // ---------------------------------------------------------------------------
  /// Establish a monitor on a process.
  Monitor(SignalMonitor),
  /// Notification that a monitored process terminated.
  MonitorDown(SignalMonitorDown),
  /// Remove a monitor on a process.
  Demonitor(SignalDemonitor),
}

impl From<SignalExit> for ControlSignal {
  #[inline]
  fn from(other: SignalExit) -> Self {
    Self::Exit(other)
  }
}

impl From<SignalLink> for ControlSignal {
  #[inline]
  fn from(other: SignalLink) -> Self {
    Self::Link(other)
  }
}

impl From<SignalLinkExit> for ControlSignal {
  #[inline]
  fn from(other: SignalLinkExit) -> Self {
    Self::LinkExit(other)
  }
}

impl From<SignalUnlink> for ControlSignal {
  #[inline]
  fn from(other: SignalUnlink) -> Self {
    Self::Unlink(other)
  }
}

impl From<SignalUnlinkAck> for ControlSignal {
  #[inline]
  fn from(other: SignalUnlinkAck) -> Self {
    Self::UnlinkAck(other)
  }
}

impl From<SignalMonitor> for ControlSignal {
  #[inline]
  fn from(other: SignalMonitor) -> Self {
    Self::Monitor(other)
  }
}

impl From<SignalMonitorDown> for ControlSignal {
  #[inline]
  fn from(other: SignalMonitorDown) -> Self {
    Self::MonitorDown(other)
  }
}

impl From<SignalDemonitor> for ControlSignal {
  #[inline]
  fn from(other: SignalDemonitor) -> Self {
    Self::Demonitor(other)
  }
}

// -----------------------------------------------------------------------------
// Signal - Exit
// -----------------------------------------------------------------------------

/// Unconditional exit signal.
///
/// Sent via `Process::exit()` to terminate a process. Processing depends
/// on the exit reason and trap_exit flag.
#[derive(Clone, Debug)]
pub(crate) struct SignalExit {
  from: LocalPid,
  exit: Exit,
}

impl SignalExit {
  #[inline]
  pub(crate) const fn new(from: LocalPid, exit: Exit) -> Self {
    Self { from, exit }
  }
}

// -----------------------------------------------------------------------------
// Signal - Link
// -----------------------------------------------------------------------------

/// Signal to establish a bidirectional link between processes.
///
/// Links enable crash propagation: when one process terminates abnormally,
/// linked processes are notified via LinkExit signals.
#[derive(Clone, Debug)]
pub(crate) struct SignalLink {
  from: LocalPid,
}

impl SignalLink {
  #[inline]
  pub(crate) const fn new(from: LocalPid) -> Self {
    Self { from }
  }
}

// -----------------------------------------------------------------------------
// Signal - LinkExit
// -----------------------------------------------------------------------------

/// Exit signal from a linked process.
///
/// Sent automatically when a linked process terminates. Processing depends
/// on the link state, exit reason, and trap_exit flag.
#[derive(Clone, Debug)]
pub(crate) struct SignalLinkExit {
  from: LocalPid,
  exit: Exit,
}

impl SignalLinkExit {
  #[inline]
  pub(crate) const fn new(from: LocalPid, exit: Exit) -> Self {
    Self { from, exit }
  }
}

// -----------------------------------------------------------------------------
// Signal - Unlink
// -----------------------------------------------------------------------------

/// Request to remove a bidirectional link.
///
/// Part of the two-phase unlink protocol. The receiver removes the link
/// and sends back UnlinkAck.
#[derive(Clone, Debug)]
pub(crate) struct SignalUnlink {
  from: LocalPid,
  ulid: NonZeroU64,
}

impl SignalUnlink {
  #[inline]
  pub(crate) const fn new(from: LocalPid, ulid: NonZeroU64) -> Self {
    Self { from, ulid }
  }
}

// -----------------------------------------------------------------------------
// Signal - UnlinkAck
// -----------------------------------------------------------------------------

/// Acknowledgment of an unlink request.
///
/// Completes the two-phase unlink protocol. The sender removes the link
/// if the unlink ID matches.
#[derive(Clone, Debug)]
pub(crate) struct SignalUnlinkAck {
  from: LocalPid,
  ulid: NonZeroU64,
}

impl SignalUnlinkAck {
  #[inline]
  pub(crate) const fn new(from: LocalPid, ulid: NonZeroU64) -> Self {
    Self { from, ulid }
  }
}

// -----------------------------------------------------------------------------
// Signal - Monitor
// -----------------------------------------------------------------------------

/// Request to monitor a process.
///
/// Establishes a unidirectional monitor. When the monitored process
/// terminates, a MonitorDown signal is sent back.
#[derive(Clone, Debug)]
pub(crate) struct SignalMonitor {
  from: LocalPid,
  mref: MonitorRef,
  item: LocalDest,
}

impl SignalMonitor {
  #[inline]
  pub(crate) const fn new(from: LocalPid, mref: MonitorRef, item: LocalDest) -> Self {
    Self { from, mref, item }
  }
}

// -----------------------------------------------------------------------------
// Signal - MonitorDown
// -----------------------------------------------------------------------------

/// Notification that a monitored process has terminated.
///
/// Sent automatically when a monitored process exits. Delivered as a
/// DOWN message to the monitoring process.
#[derive(Clone, Debug)]
pub(crate) struct SignalMonitorDown {
  from: LocalPid,
  mref: MonitorRef,
  exit: Exit,
}

impl SignalMonitorDown {
  #[inline]
  pub(crate) const fn new(from: LocalPid, mref: MonitorRef, exit: Exit) -> Self {
    Self { from, mref, exit }
  }
}

// -----------------------------------------------------------------------------
// Signal - Demonitor
// -----------------------------------------------------------------------------

/// Request to remove a monitor.
///
/// Sent when a process calls demonitor. Removes the monitor state,
/// preventing DOWN messages from being delivered.
#[derive(Clone, Debug)]
pub(crate) struct SignalDemonitor {
  from: LocalPid,
  mref: MonitorRef,
}

impl SignalDemonitor {
  #[inline]
  pub(crate) const fn new(from: LocalPid, mref: MonitorRef) -> Self {
    Self { from, mref }
  }
}
