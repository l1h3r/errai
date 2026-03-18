use tracing::Span;

use crate::core::Exit;
use crate::core::LocalPid;
use crate::core::Term;
use crate::erts::ProcInternal;
use crate::erts::ProcReadOnly;
use crate::erts::Signal;
use crate::erts::SignalEmit;
use crate::erts::SignalRecv;
use crate::erts::signal::trace_enter;
use crate::erts::signal::trace_leave;
use crate::erts::signal::trace_span;

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
    Signal::Message(self).emit(to)
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

// -----------------------------------------------------------------------------
// Signal - Send
// -----------------------------------------------------------------------------

/// Regular message signal containing user data.
///
/// Sent via `Process::send()` and delivered to the inbox for selective receive.
#[derive(Clone, Debug)]
pub(crate) struct SignalSend {
  from: LocalPid,
  data: Term,
}

impl SignalSend {
  #[inline]
  pub(crate) const fn new(from: LocalPid, data: Term) -> Self {
    Self { from, data }
  }
}

impl SignalEmit for SignalSend {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    MessageSignal::Send(self).emit(to)
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
