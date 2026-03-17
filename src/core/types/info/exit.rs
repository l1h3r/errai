use crate::core::Exit;
use crate::core::LocalPid;

/// An EXIT message from a would-be terminated process.
///
/// EXIT messages appear in the mailbox only when the receiving process
/// has the `trap_exit` flag enabled. Without this flag, EXIT signals
/// cause the receiving process to terminate instead.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ExitMessage {
  from: LocalPid,
  exit: Exit,
}

impl ExitMessage {
  /// Creates a new EXIT message.
  #[inline]
  pub(crate) fn new(from: LocalPid, exit: Exit) -> Self {
    Self { from, exit }
  }

  /// Returns the PID of the process that sent the EXIT signal.
  #[inline]
  pub const fn from(&self) -> LocalPid {
    self.from
  }

  /// Returns the exit reason.
  #[inline]
  pub const fn exit(&self) -> &Exit {
    &self.exit
  }
}
