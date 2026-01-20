use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use tokio::sync::Notify;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::error::TryRecvError;
use triomphe::Arc;

use crate::consts::CAP_PROC_MSG_BUFFER;
use crate::core::InternalPid;
use crate::erts::DynMessage;
use crate::erts::Message;
use crate::erts::Process;
use crate::erts::Signal;
use crate::proc::ProcTask;
use crate::raise;

// -----------------------------------------------------------------------------
// Proc Mail
// -----------------------------------------------------------------------------

/// Process mailbox supporting selective receive.
///
/// The mailbox stores messages in a vector and provides selective receive
/// via user-defined filter functions. Messages matching the filter are
/// removed and returned.
///
/// # Selective Receive
///
/// The `poll()` method scans the mailbox from a marker position, looking
/// for messages that match the filter. The marker tracks the scan position
/// to avoid re-scanning already-checked messages on subsequent polls.
///
/// # Notification
///
/// Each mailbox has a [`Notify`] that wakes waiters when new messages arrive.
/// This enables efficient async waiting for messages.
#[repr(C)]
pub(crate) struct ProcMail {
  mqueue: Vec<DynMessage>,
  notify: Arc<Notify>,
}

impl ProcMail {
  /// Creates a new empty mailbox.
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      mqueue: Vec::with_capacity(CAP_PROC_MSG_BUFFER),
      notify: Arc::new(Notify::new()),
    }
  }

  /// Adds a message to the mailbox and wakes all waiters.
  #[inline]
  pub(crate) fn push(&mut self, message: DynMessage) {
    self.mqueue.push(message);
    self.notify.notify_waiters();
  }

  /// Scans the mailbox for a message matching `filter`.
  ///
  /// Scanning starts at `*marker` and continues to the end. If a match is
  /// found, it's removed and returned, and `*marker` is reset to 0. If no
  /// match is found, `*marker` is updated to the mailbox length to avoid
  /// re-scanning on the next poll.
  ///
  /// # Marker Management
  ///
  /// The caller must maintain `marker` across polls to enable efficient
  /// scanning. Reset `marker` to 0 when changing filter functions.
  #[inline]
  pub(crate) fn poll<F>(&mut self, filter: F, marker: &mut usize) -> Option<DynMessage>
  where
    F: Fn(&DynMessage) -> bool,
  {
    for index in (*marker)..self.mqueue.len() {
      if filter(&self.mqueue[index]) {
        *marker = 0;
        return Some(self.mqueue.remove(index));
      }
    }

    *marker = self.mqueue.len();

    None
  }

  // ---------------------------------------------------------------------------
  // Message Polling
  // ---------------------------------------------------------------------------

  /// Receives a message of type `Message<Box<T>>`.
  ///
  /// Waits for a message matching [`DynMessage::is::<T>()`] and downcasts it.
  #[inline]
  pub(crate) async fn receive<T>(pid: InternalPid) -> Message<Box<T>>
  where
    T: 'static,
  {
    let poll: DynMessage = Self::receive_dyn(pid, DynMessage::is::<T>).await;

    // SAFETY: DynMessage::is ensures the type matches.
    let data: Message<Box<T>> = unsafe { poll.downcast_unchecked() };

    data
  }

  /// Receives an exact message of type `Box<T>` (no wrapper).
  ///
  /// Waits for a message matching [`DynMessage::is_exact::<T>()`] and
  /// downcasts it.
  #[inline]
  pub(crate) async fn receive_exact<T>(pid: InternalPid) -> Box<T>
  where
    T: 'static,
  {
    let poll: DynMessage = Self::receive_dyn(pid, DynMessage::is_exact::<T>).await;

    // SAFETY: DynMessage::is_exact ensures the type matches.
    let data: Box<T> = unsafe { poll.downcast_exact_unchecked() };

    data
  }

  /// Receives any message (always matches).
  pub(crate) async fn receive_any(pid: InternalPid) -> DynMessage {
    Self::receive_dyn(pid, |_| true).await
  }

  /// Generic receive implementation with custom filter.
  ///
  /// Polls the mailbox repeatedly until a matching message is found.
  /// Sleeps between polls using the mailbox notify signal.
  pub(crate) async fn receive_dyn<F>(pid: InternalPid, filter: F) -> DynMessage
  where
    F: Fn(&DynMessage) -> bool,
  {
    let mut marker: usize = 0;

    let mut poll = |this: &ProcTask| -> Option<DynMessage> {
      debug_assert_eq!(this.readonly.mpid, pid);
      this.internal().inbox.poll(&filter, &mut marker)
    };

    'poll: loop {
      if let Some(message) = Process::with(&mut poll) {
        break 'poll message;
      }

      Process::with(|this| Arc::clone(&this.internal().inbox.notify))
        .notified()
        .await;
    }
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
///
/// Wraps Tokio's [`UnboundedReceiver`] for receiving signals from other
/// processes. Signals are processed by the process task loop.
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
///
/// Wraps Tokio's [`UnboundedSender`] for sending signals to a process.
/// Cloneable, allowing multiple processes to send to the same target.
#[derive(Clone)]
#[repr(transparent)]
pub(crate) struct ProcSend {
  inner: UnboundedSender<Signal>,
}

impl ProcSend {
  /// Sends a signal to the process.
  ///
  /// Panics if the receiver has been dropped. This should only happen during
  /// runtime shutdown or if there's a bug in the lifecycle management.
  #[inline]
  pub(crate) fn send(&self, signal: Signal) {
    if let Err(error) = self.inner.send(signal) {
      raise!(Error, SysInv, error);
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
///
/// Returns a sender-receiver pair for process signal communication.
#[inline]
pub(crate) fn unbounded_channel() -> (ProcSend, ProcRecv) {
  let channel: _ = mpsc::unbounded_channel();
  let proc_send: ProcSend = ProcSend { inner: channel.0 };
  let proc_recv: ProcRecv = ProcRecv { inner: channel.1 };

  (proc_send, proc_recv)
}
