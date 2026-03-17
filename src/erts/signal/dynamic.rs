use crate::erts::ControlSignal;
use crate::erts::MessageSignal;

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
