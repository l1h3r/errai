use std::num::NonZeroU64;

use crate::lang::ExternalDest;
use crate::lang::InternalPid;

// -----------------------------------------------------------------------------
// Proc Link
// -----------------------------------------------------------------------------

/// State of a linked process.
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct ProcLink {
  unlink: Option<NonZeroU64>,
}

impl ProcLink {
  /// Creates a new `ProcLink`.
  #[inline]
  pub(crate) fn new() -> Self {
    Self { unlink: None }
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
}

// -----------------------------------------------------------------------------
// Proc Monitor
// -----------------------------------------------------------------------------

/// State of a monitored process.
#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcMonitor {
  origin: InternalPid,
  target: ExternalDest,
}

impl ProcMonitor {
  /// Creates a new `ProcMonitor`.
  #[inline]
  pub(crate) const fn new(origin: InternalPid, target: ExternalDest) -> Self {
    Self { origin, target }
  }

  /// Returns the monitor origin process.
  #[inline]
  pub(crate) fn origin(&self) -> InternalPid {
    self.origin
  }

  /// Returns the monitor target process.
  #[inline]
  pub(crate) fn target(&self) -> ExternalDest {
    self.target
  }
}
