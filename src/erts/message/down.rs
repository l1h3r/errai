use crate::core::Exit;
use crate::core::ExternalDest;
use crate::core::MonitorRef;

/// A message representing a monitor DOWN signal.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DownMessage {
  mref: MonitorRef,
  item: ExternalDest,
  info: Exit,
}

impl DownMessage {
  /// Creates a new `DownMessage`.
  #[inline]
  pub(crate) fn new(mref: MonitorRef, item: ExternalDest, info: Exit) -> Self {
    Self { mref, item, info }
  }

  /// Returns a reference to the monitored reference.
  #[inline]
  pub const fn mref(&self) -> &MonitorRef {
    &self.mref
  }

  /// Returns a reference to the monitored item.
  #[inline]
  pub const fn item(&self) -> &ExternalDest {
    &self.item
  }

  /// Returns the DOWN signal exit reason.
  #[inline]
  pub const fn info(&self) -> &Exit {
    &self.info
  }
}
