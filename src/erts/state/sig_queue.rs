use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use tokio::sync::Notify;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::error::TryRecvError;
use triomphe::Arc;

use crate::core::Term;
use crate::core::fatal;
use crate::erts::Signal;

// -----------------------------------------------------------------------------
// Proc Mail Constants
// -----------------------------------------------------------------------------

/// Initial capacity of the per-process internal message buffer.
const DEFAULT_CAPACITY: usize = 8;

// -----------------------------------------------------------------------------
// Proc Mail
// -----------------------------------------------------------------------------

/// Process mailbox supporting selective receive.
#[repr(C)]
pub(crate) struct ProcMail {
  mqueue: Vec<Term>,
  notify: Arc<Notify>,
}

impl ProcMail {
  /// Creates a new empty mailbox.
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      mqueue: Vec::with_capacity(DEFAULT_CAPACITY),
      notify: Arc::new(Notify::new()),
    }
  }

  /// Adds a message to the mailbox and wakes all waiters.
  #[inline]
  pub(crate) fn push(&mut self, message: Term) {
    self.mqueue.push(message);
    self.notify.notify_waiters();
  }
}

impl Debug for ProcMail {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("ProcMail ")?;
    f.debug_list().entries(self.mqueue.iter()).finish()
  }
}

// -----------------------------------------------------------------------------
// Proc Recv
// -----------------------------------------------------------------------------

/// Receiving end of the process signal queue.
#[repr(transparent)]
pub(crate) struct ProcRecv {
  inner: UnboundedReceiver<Signal>,
}

impl ProcRecv {
  /// Receives the next signal, waiting if necessary.
  ///
  /// Returns `None` if all senders have been dropped.
  #[inline]
  pub(crate) async fn recv(&mut self) -> Option<Signal> {
    self.inner.recv().await
  }

  /// Attempts to receive a signal without waiting.
  ///
  /// Returns `TryRecvError::Empty` if no signals are available.
  #[inline]
  pub(crate) fn try_recv(&mut self) -> Result<Signal, TryRecvError> {
    self.inner.try_recv()
  }
}

impl Debug for ProcRecv {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("ProcRecv(..)")
  }
}

// -----------------------------------------------------------------------------
// Proc Send
// -----------------------------------------------------------------------------

/// Sending end of the process signal queue.
#[derive(Clone)]
#[repr(transparent)]
pub(crate) struct ProcSend {
  inner: UnboundedSender<Signal>,
}

impl ProcSend {
  /// Sends a signal to the process.
  #[track_caller]
  #[inline]
  pub(crate) fn send(&self, signal: Signal) {
    if let Err(error) = self.inner.send(signal) {
      fatal!(error);
    }
  }
}

impl Debug for ProcSend {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("ProcSend(..)")
  }
}

// -----------------------------------------------------------------------------
// Misc. Utilities
// -----------------------------------------------------------------------------

/// Creates a new unbounded signal channel.
#[inline]
pub(crate) fn unbounded_channel() -> (ProcSend, ProcRecv) {
  let channel: _ = mpsc::unbounded_channel();
  let proc_send: ProcSend = ProcSend { inner: channel.0 };
  let proc_recv: ProcRecv = ProcRecv { inner: channel.1 };

  (proc_send, proc_recv)
}
