// Signal Handling
//
// # Erlang References
//
// <https://www.erlang.org/doc/system/ref_man_processes#sending-exit-signals>
// <https://www.erlang.org/doc/system/ref_man_processes#receiving-exit-signals>
// <https://www.erlang.org/doc/apps/erts/erl_dist_protocol#link_protocol>
use hashbrown::hash_map::Entry;
use std::num::NonZeroU64;
use tracing::Level;
use tracing::Span;
use tracing::span;

use crate::bifs;
use crate::core::Atom;
use crate::core::Exit;
use crate::core::ExternalDest;
use crate::core::InternalPid;
use crate::core::MonitorRef;
use crate::erts::DownMessage;
use crate::erts::DynMessage;
use crate::erts::ExitMessage;
use crate::erts::ProcessFlags;
use crate::proc::ProcInternal;
use crate::proc::ProcLink;
use crate::proc::ProcMonitor;
use crate::proc::ProcReadOnly;

macro_rules! trace_span {
  ($parent:expr, $name:expr, $($fields:tt)*) => {
    ::tracing::span!(
      target: "errai",
      parent: $parent,
      ::tracing::Level::TRACE,
      $name,
      $($fields)*
    )
  };
}

macro_rules! trace_enter {
  ($parent:expr) => {
    ::tracing::trace!(
      name: "signal",
      target: "errai",
      parent: $parent,
      action = "enter",
    );
  };
}

macro_rules! trace_leave {
  ($parent:expr, $result:expr) => {
    ::tracing::trace!(
      name: "signal",
      target: "errai",
      parent: $parent,
      action = "leave",
      result = $result,
    );
  };
}

// -----------------------------------------------------------------------------
// Signal Emit
// -----------------------------------------------------------------------------

/// Trait for sending signals to a process.
///
/// Implemented by all signal types to enable polymorphic signal sending.
/// Signals are enqueued in the target process's signal queue.
pub(crate) trait SignalEmit {
  /// Sends this signal to the target process.
  ///
  /// The signal is enqueued in the process's signal queue and will be
  /// processed asynchronously by the process task loop.
  fn emit(self, to: &ProcReadOnly);
}

// -----------------------------------------------------------------------------
// Signal Recv
// -----------------------------------------------------------------------------

/// Trait for processing received signals.
///
/// Implemented by all signal types to define their handling logic.
/// Signal processing may modify process state or trigger termination.
pub(crate) trait SignalRecv {
  /// Processes this signal in the context of the receiving process.
  ///
  /// Returns [`Exit`] if the signal should terminate the process,
  /// or [`None`] if processing completes without termination.
  ///
  /// # State Modifications
  ///
  /// Signal processing may:
  /// - Add/remove links or monitors
  /// - Enqueue messages in the inbox
  /// - Modify process flags
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit>;
}

// -----------------------------------------------------------------------------
// Signal
// -----------------------------------------------------------------------------

/// Top-level signal type wrapping message and control signals.
///
/// Signals are categorized into:
///
/// - **Message**: Regular user messages
/// - **Control**: System signals (exit, link, monitor)
#[derive(Clone, Debug)]
pub(crate) enum Signal {
  Message(MessageSignal),
  Control(ControlSignal),
}

impl SignalEmit for Signal {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    match self {
      Self::Message(signal) => signal.emit(to),
      Self::Control(signal) => signal.emit(to),
    }
  }
}

impl SignalRecv for Signal {
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    match self {
      Self::Message(signal) => signal.recv(span, readonly, internal),
      Self::Control(signal) => signal.recv(span, readonly, internal),
    }
  }
}

// -----------------------------------------------------------------------------
// Message Signal
// -----------------------------------------------------------------------------

/// Message signals containing user data.
#[derive(Clone, Debug)]
pub(crate) enum MessageSignal {
  Send(SignalSend),
}

impl SignalEmit for MessageSignal {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    match self {
      Self::Send(signal) => signal.emit(to),
    }
  }
}

impl SignalRecv for MessageSignal {
  #[inline]
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    match self {
      Self::Send(signal) => signal.recv(span, readonly, internal),
    }
  }
}

impl From<SignalSend> for MessageSignal {
  #[inline]
  fn from(other: SignalSend) -> Self {
    Self::Send(other)
  }
}

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
    match self {
      Self::Exit(signal) => signal.emit(to),
      Self::Link(signal) => signal.emit(to),
      Self::LinkExit(signal) => signal.emit(to),
      Self::Unlink(signal) => signal.emit(to),
      Self::UnlinkAck(signal) => signal.emit(to),
      Self::Monitor(signal) => signal.emit(to),
      Self::MonitorDown(signal) => signal.emit(to),
      Self::Demonitor(signal) => signal.emit(to),
    }
  }
}

impl SignalRecv for ControlSignal {
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
// Signal - Send
// -----------------------------------------------------------------------------

/// Regular message signal containing user data.
///
/// Sent via `Process::send()` and delivered to the inbox for selective receive.
#[derive(Clone, Debug)]
pub(crate) struct SignalSend {
  from: InternalPid,
  data: DynMessage,
}

impl SignalSend {
  #[inline]
  pub(crate) const fn new(from: InternalPid, data: DynMessage) -> Self {
    Self { from, data }
  }
}

impl SignalEmit for SignalSend {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Message(self.into()));
  }
}

impl SignalRecv for SignalSend {
  /// Enqueues the message in the inbox.
  ///
  /// This signal never causes termination.
  fn recv(
    self,
    span: &Span,
    _readonly: &ProcReadOnly,
    internal: &mut ProcInternal,
  ) -> Option<Exit> {
    let span: Span = trace_span!(
      span,
      "sig-send",
      from = %self.from,
    );

    trace_enter!(&span);

    internal.inbox.push(self.data);

    trace_leave!(&span, "enqueue");

    None
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
  from: InternalPid,
  exit: Exit,
}

impl SignalExit {
  #[inline]
  pub(crate) const fn new(from: InternalPid, exit: Exit) -> Self {
    Self { from, exit }
  }
}

impl SignalEmit for SignalExit {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
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
        if internal.flags.contains(ProcessFlags::TRAP_EXIT) {
          internal.send(ExitMessage::new(self.from, self.exit));
          trace_leave!(&span, "trapped");
        } else if self.from == readonly.mpid {
          trace_leave!(&span, "self-destruct");
          return Some(self.exit);
        } else {
          trace_leave!(&span, "ignored (normal)");
        }
      }
      Exit::Atom(atom) if atom == Atom::KILLED => {
        trace_leave!(&span, "terminated (killed)");
        return Some(self.exit);
      }
      Exit::Atom(_) | Exit::Term(_) => {
        if internal.flags.contains(ProcessFlags::TRAP_EXIT) {
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
  from: InternalPid,
}

impl SignalLink {
  #[inline]
  pub(crate) const fn new(from: InternalPid) -> Self {
    Self { from }
  }
}

impl SignalEmit for SignalLink {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
  }
}

impl SignalRecv for SignalLink {
  /// Establishes a link if one doesn't already exist.
  ///
  /// If a link already exists for the sender, this signal is ignored.
  /// Otherwise, a new enabled link is created.
  fn recv(
    self,
    span: &Span,
    _readonly: &ProcReadOnly,
    internal: &mut ProcInternal,
  ) -> Option<Exit> {
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
  from: InternalPid,
  exit: Exit,
}

impl SignalLinkExit {
  #[inline]
  pub(crate) const fn new(from: InternalPid, exit: Exit) -> Self {
    Self { from, exit }
  }
}

impl SignalEmit for SignalLinkExit {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
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
  fn recv(
    self,
    span: &Span,
    _readonly: &ProcReadOnly,
    internal: &mut ProcInternal,
  ) -> Option<Exit> {
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
          if internal.flags.contains(ProcessFlags::TRAP_EXIT) {
            internal.send(ExitMessage::new(self.from, self.exit));
            trace_leave!(&span, "trapped");
          } else if self.exit.is_normal() {
            trace_leave!(&span, "ignored (normal)");
          } else if self.exit.is_killed() {
            trace_leave!(&span, "terminated (killed)");
            return Some(self.exit);
          } else {
            trace_leave!(&span, "terminated (custom)");
            return Some(self.exit);
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
  from: InternalPid,
  ulid: NonZeroU64,
}

impl SignalUnlink {
  #[inline]
  pub(crate) const fn new(from: InternalPid, ulid: NonZeroU64) -> Self {
    Self { from, ulid }
  }
}

impl SignalEmit for SignalUnlink {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
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

        if let Some(proc) = bifs::proc_find(self.from) {
          proc.readonly.send_unlink_ack(readonly.mpid, self.ulid);
          trace_leave!(&span, "acknowledged");
        } else {
          trace_leave!(&span, "ignored (dead)");
        }

        entry.remove();
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
  from: InternalPid,
  ulid: NonZeroU64,
}

impl SignalUnlinkAck {
  #[inline]
  pub(crate) const fn new(from: InternalPid, ulid: NonZeroU64) -> Self {
    Self { from, ulid }
  }
}

impl SignalEmit for SignalUnlinkAck {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
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
  fn recv(
    self,
    span: &Span,
    _readonly: &ProcReadOnly,
    internal: &mut ProcInternal,
  ) -> Option<Exit> {
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
  from: InternalPid,
  mref: MonitorRef,
  item: ExternalDest,
}

impl SignalMonitor {
  #[inline]
  pub(crate) const fn new(from: InternalPid, mref: MonitorRef, item: ExternalDest) -> Self {
    Self { from, mref, item }
  }
}

impl SignalEmit for SignalMonitor {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
  }
}

impl SignalRecv for SignalMonitor {
  /// Establishes a monitor if one doesn't already exist for this reference.
  ///
  /// If a monitor with the same reference already exists, this signal is
  /// ignored. Otherwise, monitor state is created.
  fn recv(
    self,
    span: &Span,
    _readonly: &ProcReadOnly,
    internal: &mut ProcInternal,
  ) -> Option<Exit> {
    let span: Span = trace_span!(
      span,
      "sig-monitor",
      from = %self.from,
      mref = %self.mref,
      item = %self.item,
    );

    trace_enter!(&span);

    match internal.monitor_recv.entry(self.mref) {
      Entry::Occupied(_) => {
        trace_leave!(&span, "ignored (occupied)");
      }
      Entry::Vacant(entry) => {
        entry.insert(ProcMonitor::new(self.from, self.item));
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
  from: InternalPid,
  mref: MonitorRef,
  info: Exit,
}

impl SignalMonitorDown {
  #[inline]
  pub(crate) const fn new(from: InternalPid, mref: MonitorRef, info: Exit) -> Self {
    Self { from, mref, info }
  }
}

impl SignalEmit for SignalMonitorDown {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
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
      info = %self.info,
    );

    trace_enter!(&span);

    match internal.monitor_send.entry(self.mref) {
      Entry::Occupied(entry) => {
        let data: ProcMonitor = entry.remove();
        let dest: ExternalDest = data.target();

        internal.send(DownMessage::new(self.mref, dest, self.info));

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
  from: InternalPid,
  mref: MonitorRef,
}

impl SignalDemonitor {
  #[inline]
  pub(crate) const fn new(from: InternalPid, mref: MonitorRef) -> Self {
    Self { from, mref }
  }
}

impl SignalEmit for SignalDemonitor {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
  }
}

impl SignalRecv for SignalDemonitor {
  /// Removes monitor state if it exists.
  ///
  /// Ignored if no monitor state exists for the reference.
  fn recv(
    self,
    span: &Span,
    _readonly: &ProcReadOnly,
    internal: &mut ProcInternal,
  ) -> Option<Exit> {
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
