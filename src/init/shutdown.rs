use std::process;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;

use crate::raise;

// -----------------------------------------------------------------------------
// Exit Code
// -----------------------------------------------------------------------------

/// This type represents the status code the current process can return
/// to its parent under normal termination.
#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub(crate) struct ExitCode(u8);

impl ExitCode {
  /// The canonical `ExitCode` for successful termination on this platform.
  pub(crate) const SUCCESS: ExitCode = ExitCode(libc::EXIT_SUCCESS as u8);

  /// The canonical `ExitCode` for unsuccessful termination on this platform.
  pub(crate) const FAILURE: ExitCode = ExitCode(libc::EXIT_FAILURE as u8);

  /// Exit the current process with the given `ExitCode`.
  #[inline]
  pub(crate) fn exit_process(self) -> ! {
    process::exit(self.to_i32())
  }

  #[inline]
  pub(crate) fn to_i32(self) -> i32 {
    self.0 as i32
  }
}

// -----------------------------------------------------------------------------
// Termination Method
// -----------------------------------------------------------------------------

pub(crate) enum Terminate {
  // Graceful shutdown
  Stop(ExitCode),
  // Forceful shutdown
  Halt(ExitCode),
  // Forceful shutdown + crash dump
  Dump(&'static str),
}

// -----------------------------------------------------------------------------
// Termination Channel
// -----------------------------------------------------------------------------

#[inline]
pub(crate) fn channel() -> (TerminateSend, TerminateRecv) {
  let channel: _ = mpsc::channel(1);

  (
    TerminateSend { inner: channel.0 },
    TerminateRecv { inner: channel.1 },
  )
}

// -----------------------------------------------------------------------------
// Termination Channel - Send
// -----------------------------------------------------------------------------

#[derive(Clone)]
#[repr(transparent)]
pub(crate) struct TerminateSend {
  inner: Sender<Terminate>,
}

impl TerminateSend {
  #[inline]
  pub(crate) async fn send(&self, value: Terminate) {
    if let Err(error) = self.inner.send(value).await {
      raise!(Error, SysInv, error);
    }
  }
}

// -----------------------------------------------------------------------------
// Termination Channel - Recv
// -----------------------------------------------------------------------------

#[repr(transparent)]
pub(crate) struct TerminateRecv {
  inner: Receiver<Terminate>,
}

impl TerminateRecv {
  #[inline]
  pub(crate) async fn recv(&mut self) -> Option<Terminate> {
    self.inner.recv().await
  }
}
