use hashbrown::hash_map::Entry;
use std::num::NonZeroU64;
use tracing::Span;

use crate::core::Atom;
use crate::core::DownMessage;
use crate::core::Exit;
use crate::core::ExitMessage;
use crate::core::LocalDest;
use crate::core::LocalPid;
use crate::core::MonitorRef;
use crate::erts::ProcFlags;
use crate::erts::ProcInternal;
use crate::erts::ProcLink;
use crate::erts::ProcMonitor;
use crate::erts::ProcReadOnly;
use crate::erts::Signal;
use crate::erts::SignalEmit;
use crate::erts::SignalRecv;
use crate::erts::signal::trace_enter;
use crate::erts::signal::trace_leave;
use crate::erts::signal::trace_span;

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

impl SignalEmit for ControlSignal {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    Signal::Control(self).emit(to)
  }
}

impl SignalRecv for ControlSignal {
  #[inline]
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    match self {
      Self::Exit(signal) => signal.recv(span, readonly, internal),
      Self::Link(signal) => signal.recv(span, readonly, internal),
      Self::LinkExit(signal) => signal.recv(span, readonly, internal),
      Self::Unlink(signal) => signal.recv(span, readonly, internal),
      Self::UnlinkAck(signal) => signal.recv(span, readonly, internal),
      Self::Monitor(signal) => signal.recv(span, readonly, internal),
      Self::MonitorDown(signal) => signal.recv(span, readonly, internal),
      Self::Demonitor(signal) => signal.recv(span, readonly, internal),
    }
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

impl SignalEmit for SignalExit {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    ControlSignal::Exit(self).emit(to)
  }
}

impl SignalRecv for SignalExit {
  /// Processes the exit signal according to its reason and flags.
  ///
  /// # Normal Exits
  ///
  /// - **trap_exit enabled**: Converted to EXIT message
  /// - **Self-sent**: Terminates process
  /// - **Other sender**: Ignored
  ///
  /// # Kill Exits
  ///
  /// Always terminate the process (cannot be trapped).
  ///
  /// # Custom Exits
  ///
  /// - **trap_exit enabled**: Converted to EXIT message
  /// - **trap_exit disabled**: Terminates process
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    let span: Span = trace_span!(
      span,
      "sig-exit",
      from = %self.from,
      exit = %self.exit,
    );

    trace_enter!(&span);

    match self.exit {
      Exit::Atom(atom) if atom == Atom::NORMAL => {
        if internal.flags.contains(ProcFlags::TRAP_EXIT) {
          internal.send(ExitMessage::new(self.from, self.exit));
          trace_leave!(&span, "trapped");
        } else if self.from == readonly.mpid {
          trace_leave!(&span, "self-destruct");
          return Some(self.exit);
        } else {
          trace_leave!(&span, "ignored (normal)");
        }
      }
      Exit::Atom(atom) if atom == Atom::KILL => {
        trace_leave!(&span, "terminated (kill)");
        return Some(Exit::KILLED);
      }
      Exit::Atom(_) | Exit::Term(_) => {
        if internal.flags.contains(ProcFlags::TRAP_EXIT) {
          internal.send(ExitMessage::new(self.from, self.exit));
          trace_leave!(&span, "trapped");
        } else {
          trace_leave!(&span, "terminated (custom)");
          return Some(self.exit);
        }
      }
    }

    None
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

impl SignalEmit for SignalLink {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    ControlSignal::Link(self).emit(to)
  }
}

impl SignalRecv for SignalLink {
  /// Establishes a link if one doesn't already exist.
  ///
  /// If a link already exists for the sender, this signal is ignored.
  /// Otherwise, a new enabled link is created.
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    let span: Span = trace_span!(
      span,
      "sig-link",
      from = %self.from,
    );

    trace_enter!(&span);

    match internal.links.entry(self.from) {
      Entry::Occupied(_) => {
        trace_leave!(&span, "ignored (occupied)");
      }
      Entry::Vacant(entry) => {
        entry.insert(ProcLink::new());
        trace_leave!(&span, "linked");
      }
    }

    None
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

impl SignalEmit for SignalLinkExit {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    ControlSignal::LinkExit(self).emit(to)
  }
}

impl SignalRecv for SignalLinkExit {
  /// Processes exit signal from a linked process.
  ///
  /// # Processing Rules
  ///
  /// Requires an active (enabled) link to the sender:
  ///
  /// - **trap_exit enabled**: Converted to EXIT message
  /// - **Normal exit**: Ignored (doesn't propagate)
  /// - **Kill exit**: Terminates process
  /// - **Custom exit**: Terminates process
  ///
  /// Signals are ignored if:
  ///
  /// - No link exists
  /// - Link is disabled (unlink in progress)
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    let span: Span = trace_span!(
      span,
      "sig-link-exit",
      from = %self.from,
      exit = %self.exit,
    );

    trace_enter!(&span);

    match internal.links.entry(self.from) {
      Entry::Occupied(entry) => {
        if entry.get().is_enabled() {
          if internal.flags.contains(ProcFlags::TRAP_EXIT) {
            internal.send(ExitMessage::new(self.from, self.exit));
            trace_leave!(&span, "trapped");
          } else {
            match self.exit {
              Exit::Atom(atom) if atom == Atom::NORMAL => {
                trace_leave!(&span, "ignored (normal)");
              }
              Exit::Atom(atom) if atom == Atom::KILL => {
                trace_leave!(&span, "terminated (kill)");
                return Some(self.exit);
              }
              Exit::Atom(_) | Exit::Term(_) => {
                trace_leave!(&span, "terminated (custom)");
                return Some(self.exit);
              }
            }
          }
        } else {
          trace_leave!(&span, "disabled");
        }
      }
      Entry::Vacant(_) => {
        trace_leave!(&span, "ignored (vacant)");
      }
    }

    None
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

impl SignalEmit for SignalUnlink {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    ControlSignal::Unlink(self).emit(to)
  }
}

impl SignalRecv for SignalUnlink {
  /// Processes unlink request and sends acknowledgment.
  ///
  /// If an enabled link exists:
  ///
  /// 1. Sends UnlinkAck back to the sender (if sender still exists)
  /// 2. Removes the link
  ///
  /// Signals are ignored if:
  ///
  /// - No link exists
  /// - Link is already disabled
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    let span: Span = trace_span!(
      span,
      "sig-unlink",
      from = %self.from,
      ulid = %self.ulid,
    );

    trace_enter!(&span);

    match internal.links.entry(self.from) {
      Entry::Occupied(entry) => {
        if entry.get().is_disabled() {
          trace_leave!(&span, "disabled");
          return None;
        }

        entry.remove();

        // TODO: Send Ack
      }
      Entry::Vacant(_) => {
        trace_leave!(&span, "ignored (vacant)");
      }
    }

    None
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

impl SignalEmit for SignalUnlinkAck {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    ControlSignal::UnlinkAck(self).emit(to)
  }
}

impl SignalRecv for SignalUnlinkAck {
  /// Completes the unlink if the ID matches.
  ///
  /// The link is removed only if:
  ///
  /// - A disabled link exists for the sender
  /// - The unlink ID matches the stored ID
  ///
  /// This prevents removing a link if:
  ///
  /// - The link was re-enabled
  /// - A stale acknowledgment arrives
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    let span: Span = trace_span!(
      span,
      "sig-unlink-ack",
      from = %self.from,
      ulid = %self.ulid,
    );

    trace_enter!(&span);

    match internal.links.entry(self.from) {
      Entry::Occupied(entry) => {
        if entry.get().is_disabled() {
          if entry.get().matches(self.ulid) {
            entry.remove();
            trace_leave!(&span, "unlinked");
          } else {
            trace_leave!(&span, "ignored (stale)");
          }
        } else {
          trace_leave!(&span, "ignored (enabled)");
        }
      }
      Entry::Vacant(_) => {
        trace_leave!(&span, "ignored (vacant)");
      }
    }

    None
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
  dest: LocalDest,
}

impl SignalMonitor {
  #[inline]
  pub(crate) const fn new(from: LocalPid, mref: MonitorRef, dest: LocalDest) -> Self {
    Self { from, mref, dest }
  }
}

impl SignalEmit for SignalMonitor {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    ControlSignal::Monitor(self).emit(to)
  }
}

impl SignalRecv for SignalMonitor {
  /// Establishes a monitor if one doesn't already exist for this reference.
  ///
  /// If a monitor with the same reference already exists, this signal is
  /// ignored. Otherwise, monitor state is created.
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    let span: Span = trace_span!(
      span,
      "sig-monitor",
      from = %self.from,
      mref = %self.mref,
      dest = %self.dest,
    );

    trace_enter!(&span);

    match internal.monitor_recv.entry(self.mref) {
      Entry::Occupied(_) => {
        trace_leave!(&span, "ignored (occupied)");
      }
      Entry::Vacant(entry) => {
        entry.insert(ProcMonitor::new(self.from, self.dest));
        trace_leave!(&span, "monitored");
      }
    }

    None
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

impl SignalEmit for SignalMonitorDown {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    ControlSignal::MonitorDown(self).emit(to)
  }
}

impl SignalRecv for SignalMonitorDown {
  /// Delivers DOWN message and removes monitor state.
  ///
  /// If monitor state exists for the reference:
  ///
  /// 1. Sends DOWN message to the monitoring process
  /// 2. Removes the monitor state
  ///
  /// Ignored if no monitor state exists (monitor was removed).
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    let span: Span = trace_span!(
      span,
      "sig-monitor-down",
      from = %self.from,
      mref = %self.mref,
      exit = %self.exit,
    );

    trace_enter!(&span);

    match internal.monitor_send.entry(self.mref) {
      Entry::Occupied(entry) => {
        let data: ProcMonitor = entry.remove();
        let dest: LocalDest = data.target();

        internal.send(DownMessage::new(self.mref, dest, self.exit));

        trace_leave!(&span, "trapped");
      }
      Entry::Vacant(_) => {
        trace_leave!(&span, "ignored (vacant)");
      }
    }

    None
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

impl SignalEmit for SignalDemonitor {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    ControlSignal::Demonitor(self).emit(to)
  }
}

impl SignalRecv for SignalDemonitor {
  /// Removes monitor state if it exists.
  ///
  /// Ignored if no monitor state exists for the reference.
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    let span: Span = trace_span!(
      span,
      "sig-demonitor",
      from = %self.from,
      mref = %self.mref,
    );

    trace_enter!(&span);

    match internal.monitor_recv.entry(self.mref) {
      Entry::Occupied(entry) => {
        entry.remove();
        trace_leave!(&span, "demonitored");
      }
      Entry::Vacant(_) => {
        trace_leave!(&span, "ignored (vacant)");
      }
    }

    None
  }
}
