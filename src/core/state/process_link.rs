use std::num::NonZeroU64;

use crate::core::ProcSend;
use crate::core::WeakProcSend;

/// State of a linked process.
#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcLink {
  sender: WeakProcSend,
  unlink: Option<NonZeroU64>,
}

impl ProcLink {
  /// Creates a new `ProcLink`.
  #[inline]
  pub(crate) fn new(sender: WeakProcSend) -> Self {
    Self {
      sender,
      unlink: None,
    }
  }

  /// Returns `true` if the link is enabled.
  #[inline]
  pub fn is_enabled(&self) -> bool {
    self.unlink.is_none()
  }

  /// Returns `true` if the link is disabled.
  #[inline]
  pub fn is_disabled(&self) -> bool {
    !self.is_enabled()
  }

  /// Sets the link state to "enabled".
  #[inline]
  pub(crate) fn enable(&mut self) {
    self.unlink = None;
  }

  /// Sets the link state to "disabled".
  #[inline]
  pub(crate) fn disable(&mut self, unlink: NonZeroU64) {
    self.unlink = Some(unlink);
  }

  /// Returns `true` if the link state matches the given unlink id.
  #[inline]
  pub(crate) fn matches(&self, ulid: NonZeroU64) -> bool {
    self.unlink.map(|id| id == ulid).unwrap_or(false)
  }

  /// Tries to convert the associated `WeakProcSend` into a `ProcSend`.
  ///
  /// Returns `Some` if the channel is still open, otherwise `None`.
  #[inline]
  pub(crate) fn sender(&self) -> Option<ProcSend> {
    self.sender.upgrade()
  }
}
