use crate::core::Exit;
use crate::core::InternalPid;

/// A message representing a trapped EXIT signal.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ExitMessage {
  from: InternalPid,
  exit: Exit,
}

impl ExitMessage {
  /// Creates a new `ExitMessage`.
  #[inline]
  pub(crate) fn new(from: InternalPid, exit: Exit) -> Self {
    Self {
      from: from.into(),
      exit,
    }
  }

  /// Returns a reference to the EXIT signal sender.
  #[inline]
  pub const fn from(&self) -> InternalPid {
    self.from
  }

  /// Returns the EXIT signal exit reason.
  #[inline]
  pub const fn exit(&self) -> &Exit {
    &self.exit
  }
}
