use tracing::Span;

use crate::core::Exit;
use crate::erts::ControlSignal;
use crate::erts::MessageSignal;
use crate::erts::ProcInternal;
use crate::erts::ProcReadOnly;
use crate::erts::SignalEmit;
use crate::erts::SignalRecv;

// -----------------------------------------------------------------------------
// Signal
// -----------------------------------------------------------------------------

/// Top-level signal type wrapping message and control signals.
///
/// Signals are categorized into:
///
/// - **Control**: System signals (exit, link, monitor)
/// - **Message**: Regular user messages
#[derive(Clone, Debug)]
pub(crate) enum Signal {
  Control(ControlSignal),
  Message(MessageSignal),
}

impl SignalEmit for Signal {
  #[inline]
  fn emit(self, to: &ProcReadOnly) {
    match self {
      Self::Control(signal) => signal.emit(to),
      Self::Message(signal) => signal.emit(to),
    }
  }
}

impl SignalRecv for Signal {
  #[inline]
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit> {
    match self {
      Self::Control(signal) => signal.recv(span, readonly, internal),
      Self::Message(signal) => signal.recv(span, readonly, internal),
    }
  }
}

impl From<ControlSignal> for Signal {
  #[inline]
  fn from(other: ControlSignal) -> Self {
    Self::Control(other)
  }
}

impl From<MessageSignal> for Signal {
  #[inline]
  fn from(other: MessageSignal) -> Self {
    Self::Message(other)
  }
}
