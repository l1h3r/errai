use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use tokio::sync::mpsc::error::TryRecvError;

use crate::erts::DynMessage;
use crate::erts::ProcessRecv;
use crate::erts::ProcessSend;
use crate::erts::Runtime;
use crate::erts::Signal;

// -----------------------------------------------------------------------------
// Disconnected Error
// -----------------------------------------------------------------------------

/// Error returned by [`SignalQueue::poll`].
#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct Disconnected;

impl Display for Disconnected {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("receiving on a closed channel")
  }
}

impl Error for Disconnected {}

// -----------------------------------------------------------------------------
// Signal Queue
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct SignalQueue {
  internal: Vec<DynMessage>,
  external: ProcessRecv,
}

impl SignalQueue {
  pub(crate) fn new() -> (ProcessSend, Self) {
    let channel: (ProcessSend, ProcessRecv) = super::unbounded_channel();

    let queue: Self = Self {
      internal: Vec::with_capacity(Runtime::CAP_SIG_QUEUE_INTERNAL),
      external: channel.1,
    };

    (channel.0, queue)
  }

  pub(crate) fn poll_internal<F>(&mut self, filter: F) -> Option<DynMessage>
  where
    F: Fn(&DynMessage) -> bool,
  {
    self
      .internal
      .iter()
      .position(filter)
      .map(|index| self.internal.remove(index))
  }

  pub(crate) fn poll_external<F>(&mut self, filter: F) -> Result<Option<DynMessage>, Disconnected>
  where
    F: Fn(&DynMessage) -> bool,
  {
    'poll: loop {
      let signal: Signal = match self.external.try_recv() {
        Ok(signal) => signal,
        Err(TryRecvError::Empty) => break 'poll Ok(None),
        Err(TryRecvError::Disconnected) => break 'poll Err(Disconnected),
      };

      let Some(message) = self.capture(signal) else {
        continue 'poll;
      };

      if filter(&message) {
        break 'poll Ok(Some(message));
      }

      self.internal.push(message);
    }
  }

  fn capture(&mut self, input: Signal) -> Option<DynMessage> {
    match input {
      Signal::Message(_from, term) => Some(DynMessage::Term(term)),
    }
  }
}
