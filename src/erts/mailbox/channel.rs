use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::error::TryRecvError;

use crate::erts::Signal;

#[inline]
pub(crate) fn unbounded_channel() -> (ProcessSend, ProcessRecv) {
  let channel: _ = mpsc::unbounded_channel();

  (
    ProcessSend { inner: channel.0 },
    ProcessRecv { inner: channel.1 },
  )
}

// -----------------------------------------------------------------------------
// Process Send
// -----------------------------------------------------------------------------

#[repr(transparent)]
pub(crate) struct ProcessSend {
  inner: UnboundedSender<Signal>,
}

impl ProcessSend {
  #[inline]
  pub(crate) fn send(&self, signal: Signal) -> Result<(), SendError<Signal>> {
    self.inner.send(signal)
  }
}

impl Debug for ProcessSend {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("ProcessSend(..)")
  }
}

// -----------------------------------------------------------------------------
// Process Recv
// -----------------------------------------------------------------------------

#[repr(transparent)]
pub(crate) struct ProcessRecv {
  inner: UnboundedReceiver<Signal>,
}

impl ProcessRecv {
  #[inline]
  pub(crate) async fn recv(&mut self) -> Option<Signal> {
    self.inner.recv().await
  }

  #[inline]
  pub(crate) fn try_recv(&mut self) -> Result<Signal, TryRecvError> {
    self.inner.try_recv()
  }
}

impl Debug for ProcessRecv {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("ProcessRecv(..)")
  }
}
