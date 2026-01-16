use hashbrown::hash_map::Entry;
use std::num::NonZeroU64;
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

// -----------------------------------------------------------------------------
// Signal Emit
// -----------------------------------------------------------------------------

pub(crate) trait SignalEmit {
  fn emit(self, to: &ProcReadOnly);
}

// -----------------------------------------------------------------------------
// Signal Recv
// -----------------------------------------------------------------------------

pub(crate) trait SignalRecv {
  fn recv(self, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit>;
}

// -----------------------------------------------------------------------------
// Signal
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) enum Signal {
  Message(MessageSignal),
  Control(ControlSignal),
}

impl Signal {
  #[inline]
  const fn kind(&self) -> &'static str {
    match self {
      Self::Message(_) => "M",
      Self::Control(_) => "C",
    }
  }

  #[inline]
  const fn from(&self) -> InternalPid {
    match self {
      Self::Message(signal) => signal.from(),
      Self::Control(signal) => signal.from(),
    }
  }
}

impl SignalEmit for Signal {
  fn emit(self, to: &ProcReadOnly) {
    match self {
      Self::Message(signal) => signal.emit(to),
      Self::Control(signal) => signal.emit(to),
    }
  }
}

impl SignalRecv for Signal {
  fn recv(self, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    let span: Span = tracing::trace_span!(
      "Proc Signal",
      type = %self.kind(),
      from = %self.from(),
    );

    let _enter: span::Entered<'_> = span.enter();

    match self {
      Self::Message(signal) => signal.recv(readonly, internal),
      Self::Control(signal) => signal.recv(readonly, internal),
    }
  }
}

// -----------------------------------------------------------------------------
// Message Signal
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) enum MessageSignal {
  Send(SignalSend),
}

impl MessageSignal {
  #[inline]
  const fn from(&self) -> InternalPid {
    match self {
      Self::Send(signal) => signal.from(),
    }
  }
}

impl SignalEmit for MessageSignal {
  fn emit(self, to: &ProcReadOnly) {
    match self {
      Self::Send(signal) => signal.emit(to),
    }
  }
}

impl SignalRecv for MessageSignal {
  fn recv(self, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    match self {
      Self::Send(signal) => signal.recv(readonly, internal),
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

#[derive(Clone, Debug)]
pub(crate) enum ControlSignal {
  // ---------------------------------------------------------------------------
  // Termination Signals
  // ---------------------------------------------------------------------------
  Exit(SignalExit),
  // ---------------------------------------------------------------------------
  // Link Signals
  // ---------------------------------------------------------------------------
  Link(SignalLink),
  LinkExit(SignalLinkExit),
  Unlink(SignalUnlink),
  UnlinkAck(SignalUnlinkAck),
  // ---------------------------------------------------------------------------
  // Monitor Signals
  // ---------------------------------------------------------------------------
  Monitor(SignalMonitor),
  MonitorDown(SignalMonitorDown),
  Demonitor(SignalDemonitor),
}

impl ControlSignal {
  #[inline]
  const fn from(&self) -> InternalPid {
    match self {
      Self::Exit(signal) => signal.from(),
      Self::Link(signal) => signal.from(),
      Self::LinkExit(signal) => signal.from(),
      Self::Unlink(signal) => signal.from(),
      Self::UnlinkAck(signal) => signal.from(),
      Self::Monitor(signal) => signal.from(),
      Self::MonitorDown(signal) => signal.from(),
      Self::Demonitor(signal) => signal.from(),
    }
  }
}

impl SignalEmit for ControlSignal {
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
  fn recv(self, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    match self {
      Self::Exit(signal) => signal.recv(readonly, internal),
      Self::Link(signal) => signal.recv(readonly, internal),
      Self::LinkExit(signal) => signal.recv(readonly, internal),
      Self::Unlink(signal) => signal.recv(readonly, internal),
      Self::UnlinkAck(signal) => signal.recv(readonly, internal),
      Self::Monitor(signal) => signal.recv(readonly, internal),
      Self::MonitorDown(signal) => signal.recv(readonly, internal),
      Self::Demonitor(signal) => signal.recv(readonly, internal),
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

// ---------------------------------------------------------------------------
// Signal - Send
// ---------------------------------------------------------------------------

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

  #[inline]
  const fn from(&self) -> InternalPid {
    self.from
  }
}

impl SignalEmit for SignalSend {
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Message(self.into()));
  }
}

// Send signal handling:
//
// The content of the message is moved to the internal inbox buffer.
impl SignalRecv for SignalSend {
  fn recv(self, _readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    tracing::trace!(signal = "send");

    internal.inbox.push(self.data);

    tracing::trace!(result = "enqueue");

    None
  }
}

// ---------------------------------------------------------------------------
// Signal - Exit
// ---------------------------------------------------------------------------

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

  #[inline]
  const fn from(&self) -> InternalPid {
    self.from
  }
}

impl SignalEmit for SignalExit {
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
  }
}

impl SignalRecv for SignalExit {
  fn recv(self, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    tracing::trace!(signal = "exit", exit = %self.exit);

    match self.exit {
      Exit::Atom(atom) if atom == Atom::NORMAL => {
        if internal.flags.contains(ProcessFlags::TRAP_EXIT) {
          readonly.send_message(self.from, ExitMessage::new(self.from, self.exit));
          tracing::trace!(result = "trapped", reason = "proc flag");
        } else if self.from == readonly.mpid {
          tracing::trace!(result = "terminated", reason = "self-destruct");
          return Some(self.exit);
        } else {
          tracing::trace!(result = "ignored");
        }
      }
      Exit::Atom(atom) if atom == Atom::KILLED => {
        tracing::trace!(result = "terminated", reason = "killed");
        return Some(self.exit);
      }
      Exit::Atom(_) | Exit::Term(_) => {
        if internal.flags.contains(ProcessFlags::TRAP_EXIT) {
          readonly.send_message(self.from, ExitMessage::new(self.from, self.exit));
          tracing::trace!(result = "trapped", reason = "proc flag");
        } else {
          tracing::trace!(result = "terminated", reason = "custom");
          return Some(self.exit);
        }
      }
    }

    None
  }
}

// ---------------------------------------------------------------------------
// Signal - Link
// ---------------------------------------------------------------------------

// Link signal handling:
//
// If no link state exists for the sender, a default enabled state is inserted;
// otherwise, the signal is dropped.
#[derive(Clone, Debug)]
pub(crate) struct SignalLink {
  from: InternalPid,
}

impl SignalLink {
  #[inline]
  pub(crate) const fn new(from: InternalPid) -> Self {
    Self { from }
  }

  #[inline]
  const fn from(&self) -> InternalPid {
    self.from
  }
}

impl SignalEmit for SignalLink {
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
  }
}

impl SignalRecv for SignalLink {
  fn recv(self, _readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    tracing::trace!(signal = "link");

    match internal.links.entry(self.from) {
      Entry::Occupied(_) => {
        tracing::trace!(result = "ignored", reason = "old link");
      }
      Entry::Vacant(entry) => {
        entry.insert(ProcLink::new());
        tracing::trace!(result = "handled", reason = "new link");
      }
    }

    None
  }
}

// ---------------------------------------------------------------------------
// Signal - LinkExit
// ---------------------------------------------------------------------------

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

  #[inline]
  const fn from(&self) -> InternalPid {
    self.from
  }
}

impl SignalEmit for SignalLinkExit {
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
  }
}

// Exit signal handling (linked):
//
// If the link state exists and the process is trapping exits, the signal is
// converted to an `ExitMessage` and delivered to the inbox. If the reason is
// not normal, the receiving process is terminated.
impl SignalRecv for SignalLinkExit {
  fn recv(self, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    tracing::trace!(signal = "link exit", exit = %self.exit);

    match internal.links.entry(self.from) {
      Entry::Occupied(entry) => {
        if entry.get().is_enabled() {
          if internal.flags.contains(ProcessFlags::TRAP_EXIT) {
            readonly.send_message(self.from, ExitMessage::new(self.from, self.exit));
            tracing::trace!(result = "trapped", reason = "proc flag");
          } else if self.exit.is_normal() {
            tracing::trace!(result = "ignored", reason = "normal");
          } else if self.exit.is_killed() {
            tracing::trace!(result = "terminated", reason = "killed");
            return Some(self.exit);
          } else {
            tracing::trace!(result = "terminated", reason = "custom");
            return Some(self.exit);
          }
        } else {
          tracing::trace!(result = "ignored", reason = "link disabled");
        }
      }
      Entry::Vacant(_) => {
        tracing::trace!(result = "ignored", reason = "no link");
      }
    }

    None
  }
}

// ---------------------------------------------------------------------------
// Signal - Unlink
// ---------------------------------------------------------------------------

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

  #[inline]
  const fn from(&self) -> InternalPid {
    self.from
  }
}

impl SignalEmit for SignalUnlink {
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
  }
}

// Unlink signal handling:
//
// If the link state exists and is enabled, it is removed and the sender
// receives an `UnlinkAck` signal. If not, the signal is dropped.
impl SignalRecv for SignalUnlink {
  fn recv(self, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    tracing::trace!(signal = "unlink", ulid = %self.ulid);

    match internal.links.entry(self.from) {
      Entry::Occupied(entry) => {
        if entry.get().is_disabled() {
          tracing::trace!(result = "ignored", reason = "link disabled");
          return None;
        }

        if let Some(proc) = bifs::process_find(self.from) {
          proc.readonly.send_unlink_ack(readonly.mpid, self.ulid);
          tracing::trace!(result = "handled", reason = "good PID");
        } else {
          tracing::trace!(result = "ignored", reason = "dead PID");
        }

        entry.remove();
      }
      Entry::Vacant(_) => {
        tracing::trace!(result = "ignored", reason = "no link");
      }
    }

    None
  }
}

// ---------------------------------------------------------------------------
// Signal - UnlinkAck
// ---------------------------------------------------------------------------

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

  #[inline]
  const fn from(&self) -> InternalPid {
    self.from
  }
}

impl SignalEmit for SignalUnlinkAck {
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
  }
}

// UnlinkAck signal handling:
//
// If the link state exists, is disabled, and the given `ulid` matches, the link
// state is removed. Otherwise, the signal is dropped.
impl SignalRecv for SignalUnlinkAck {
  fn recv(self, _readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    tracing::trace!(signal = "unlink ack", ulid = %self.ulid);

    match internal.links.entry(self.from) {
      Entry::Occupied(entry) => {
        if entry.get().is_disabled() {
          if entry.get().matches(self.ulid) {
            entry.remove();
            tracing::trace!(result = "handled", reason = "fresh ulid");
          } else {
            tracing::trace!(result = "ignored", reason = "stale ulid");
          }
        } else {
          tracing::trace!(result = "ignored", reason = "link enabled");
        }
      }
      Entry::Vacant(_) => {
        tracing::trace!(result = "ignored", reason = "no link");
      }
    }

    None
  }
}

// ---------------------------------------------------------------------------
// Signal - Monitor
// ---------------------------------------------------------------------------

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

  #[inline]
  const fn from(&self) -> InternalPid {
    self.from
  }
}

impl SignalEmit for SignalMonitor {
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
  }
}

// Monitor signal handling:
//
// If the monitor state does not exist, a default state is inserted;
// otherwise, the signal is dropped.
impl SignalRecv for SignalMonitor {
  fn recv(self, _readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    tracing::trace!(signal = "monitor", mref = %self.mref, item = %self.item);

    match internal.monitor_recv.entry(self.mref) {
      Entry::Occupied(_) => {
        tracing::trace!(result = "ignored", reason = "occupied");
      }
      Entry::Vacant(entry) => {
        entry.insert(ProcMonitor::new(self.from, self.item));
        tracing::trace!(result = "handled");
      }
    }

    None
  }
}

// ---------------------------------------------------------------------------
// Signal - MonitorDown
// ---------------------------------------------------------------------------

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

  #[inline]
  const fn from(&self) -> InternalPid {
    self.from
  }
}

impl SignalEmit for SignalMonitorDown {
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
  }
}

// MonitorDown signal handling:
//
// If the monitor state exists, it is removed and the signal is sent to the
// internal inbox buffer. If not, the signal is dropped.
impl SignalRecv for SignalMonitorDown {
  fn recv(self, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    tracing::trace!(signal = "monitor down", mref = %self.mref);

    match internal.monitor_send.entry(self.mref) {
      Entry::Occupied(entry) => {
        let state: &ProcMonitor = entry.get();

        readonly.send_message(
          state.origin(),
          DownMessage::new(self.mref, state.target(), self.info),
        );

        entry.remove();

        tracing::trace!(result = "handled", reason = "good mref");
      }
      Entry::Vacant(_) => {
        tracing::trace!(result = "ignored", reason = "no monitor");
      }
    }

    None
  }
}

// ---------------------------------------------------------------------------
// Signal - Demonitor
// ---------------------------------------------------------------------------

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

  #[inline]
  const fn from(&self) -> InternalPid {
    self.from
  }
}

impl SignalEmit for SignalDemonitor {
  fn emit(self, to: &ProcReadOnly) {
    to.send.send(Signal::Control(self.into()));
  }
}

impl SignalRecv for SignalDemonitor {
  fn recv(self, _readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    tracing::trace!(signal = "demonitor", mref = %self.mref);

    match internal.monitor_recv.entry(self.mref) {
      Entry::Occupied(entry) => {
        entry.remove();
        tracing::trace!(result = "handled", reason = "good mref");
      }
      Entry::Vacant(_) => {
        tracing::trace!(result = "ignored", reason = "no monitor");
      }
    }

    None
  }
}
