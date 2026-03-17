use crate::core::LocalPid;
use crate::core::Term;

// -----------------------------------------------------------------------------
// Message Signal
// -----------------------------------------------------------------------------

/// Message signals containing user data.
#[derive(Clone, Debug)]
pub(crate) enum MessageSignal {
  Send(SignalSend),
}

impl From<SignalSend> for MessageSignal {
  #[inline]
  fn from(other: SignalSend) -> Self {
    Self::Send(other)
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
