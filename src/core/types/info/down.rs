use crate::core::Exit;
use crate::core::LocalDest;
use crate::core::MonitorRef;

/// A DOWN message sent when a monitored process terminates.
///
/// DOWN messages are delivered to processes that have established a
/// monitor on another process. They contain the monitor reference,
/// the monitored destination, and the exit reason.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DownMessage {
  mref: MonitorRef,
  dest: LocalDest,
  exit: Exit,
}

impl DownMessage {
  /// Creates a new DOWN message.
  #[inline]
  pub(crate) fn new(mref: MonitorRef, dest: LocalDest, exit: Exit) -> Self {
    Self { mref, dest, exit }
  }

  /// Returns the monitor reference.
  ///
  /// This matches the reference returned when the monitor was created.
  #[inline]
  pub const fn mref(&self) -> &MonitorRef {
    &self.mref
  }

  /// Returns the monitored destination.
  ///
  /// This is the process or registered name that was being monitored.
  #[inline]
  pub const fn dest(&self) -> &LocalDest {
    &self.dest
  }

  /// Returns the exit reason of the monitored process.
  #[inline]
  pub const fn exit(&self) -> &Exit {
    &self.exit
  }
}
