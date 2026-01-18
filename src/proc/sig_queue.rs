use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use tokio::sync::Notify;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::WeakUnboundedSender;
use tokio::sync::mpsc::error::TryRecvError;
use triomphe::Arc;

use crate::consts::CAP_PROC_MSG_BUFFER;
use crate::core::InternalPid;
use crate::core::raise;
use crate::erts::DynMessage;
use crate::erts::Message;
use crate::erts::Process;
use crate::erts::Signal;
use crate::proc::ProcTask;

// -----------------------------------------------------------------------------
// Proc Mail
// -----------------------------------------------------------------------------

#[repr(C)]
pub(crate) struct ProcMail {
  mqueue: Vec<DynMessage>,
  notify: Arc<Notify>,
}

impl ProcMail {
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      mqueue: Vec::with_capacity(CAP_PROC_MSG_BUFFER),
      notify: Arc::new(Notify::new()),
    }
  }

  #[inline]
  pub(crate) fn push(&mut self, message: DynMessage) {
    self.mqueue.push(message);
    self.notify.notify_waiters();
  }

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

  #[inline]
  pub(crate) async fn receive<T>(pid: InternalPid) -> Message<Box<T>>
  where
    T: 'static,
  {
    let poll: DynMessage = Self::receive_dyn(pid, DynMessage::is::<T>).await;

    // SAFETY: `DynMessage::is` ensures the message type is valid.
    let data: Message<Box<T>> = unsafe { poll.downcast_unchecked() };

    data
  }

  #[inline]
  pub(crate) async fn receive_exact<T>(pid: InternalPid) -> Box<T>
  where
    T: 'static,
  {
    let poll: DynMessage = Self::receive_dyn(pid, DynMessage::is_exact::<T>).await;

    // SAFETY: `DynMessage::is_exact` ensures the message type is valid.
    let data: Box<T> = unsafe { poll.downcast_exact_unchecked() };

    data
  }

  pub(crate) async fn receive_any(pid: InternalPid) -> DynMessage {
    Self::receive_dyn(pid, |_| true).await
  }

  pub(crate) async fn receive_dyn<F>(pid: InternalPid, filter: F) -> DynMessage
  where
    F: Fn(&DynMessage) -> bool,
  {
    let mut marker: usize = 0;

    let mut poll = |this: &ProcTask| -> Option<DynMessage> {
      debug_assert_eq!(this.readonly.mpid, pid);
      this.internal.lock().inbox.poll(&filter, &mut marker)
    };

    'poll: loop {
      if let Some(message) = Process::with(&mut poll) {
        break 'poll message;
      }

      Process::with(|this| Arc::clone(&this.internal.lock().inbox.notify))
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

#[repr(transparent)]
pub(crate) struct ProcRecv {
  inner: UnboundedReceiver<Signal>,
}

impl ProcRecv {
  #[inline]
  pub(crate) async fn recv(&mut self) -> Option<Signal> {
    self.inner.recv().await
  }

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

#[derive(Clone)]
#[repr(transparent)]
pub(crate) struct ProcSend {
  inner: UnboundedSender<Signal>,
}

impl ProcSend {
  #[inline]
  pub(crate) fn send(&self, signal: Signal) {
    if let Err(error) = self.inner.send(signal) {
      raise!(Error, SysInv, error);
    }
  }

  #[inline]
  pub(crate) fn downgrade(&self) -> WeakProcSend {
    WeakProcSend {
      inner: self.inner.downgrade(),
    }
  }
}

impl Debug for ProcSend {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("ProcSend(..)")
  }
}

// -----------------------------------------------------------------------------
// Weak Proc Send
// -----------------------------------------------------------------------------

#[derive(Clone)]
#[repr(transparent)]
pub(crate) struct WeakProcSend {
  inner: WeakUnboundedSender<Signal>,
}

impl WeakProcSend {
  #[inline]
  pub(crate) fn upgrade(&self) -> Option<ProcSend> {
    self.inner.upgrade().map(|inner| ProcSend { inner })
  }
}

impl Debug for WeakProcSend {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("WeakProcSend(..)")
  }
}

// -----------------------------------------------------------------------------
// Misc. Utilities
// -----------------------------------------------------------------------------

#[inline]
pub(crate) fn unbounded_channel() -> (ProcSend, ProcRecv) {
  let channel: _ = mpsc::unbounded_channel();
  let proc_send: ProcSend = ProcSend { inner: channel.0 };
  let proc_recv: ProcRecv = ProcRecv { inner: channel.1 };

  (proc_send, proc_recv)
}
