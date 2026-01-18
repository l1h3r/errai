use crate::core::Exit;
use crate::core::ExternalDest;
use crate::core::MonitorRef;

/// A DOWN message sent when a monitored process terminates.
///
/// DOWN messages are delivered to processes that have established a
/// monitor on another process. They contain the monitor reference,
/// the monitored destination, and the exit reason.
///
/// # Fields
///
/// - `mref`: The monitor reference returned when the monitor was created
/// - `item`: The monitored destination (PID or registered name)
/// - `info`: The exit reason of the terminated process
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DownMessage {
  mref: MonitorRef,
  item: ExternalDest,
  info: Exit,
}

impl DownMessage {
  /// Creates a new DOWN message.
  #[inline]
  pub(crate) fn new(mref: MonitorRef, item: ExternalDest, info: Exit) -> Self {
    Self { mref, item, info }
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
  pub const fn item(&self) -> &ExternalDest {
    &self.item
  }

  /// Returns the exit reason of the monitored process.
  #[inline]
  pub const fn info(&self) -> &Exit {
    &self.info
  }
}
