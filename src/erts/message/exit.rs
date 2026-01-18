use crate::core::Exit;
use crate::core::InternalPid;

/// An EXIT message from a would-be terminated process.
///
/// EXIT messages appear in the mailbox only when the receiving process
/// has the `trap_exit` flag enabled. Without this flag, EXIT signals
/// cause the receiving process to terminate instead.
///
/// # Fields
///
/// - `from`: The PID of the process that sent the EXIT signal
/// - `exit`: The exit reason
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ExitMessage {
  from: InternalPid,
  exit: Exit,
}

impl ExitMessage {
  /// Creates a new EXIT message.
  #[inline]
  pub(crate) fn new(from: InternalPid, exit: Exit) -> Self {
    Self {
      from: from.into(),
      exit,
    }
  }

  /// Returns the PID of the process that sent the EXIT signal.
  #[inline]
  pub const fn from(&self) -> InternalPid {
    self.from
  }

  /// Returns the exit reason.
  #[inline]
  pub const fn exit(&self) -> &Exit {
    &self.exit
  }
}
