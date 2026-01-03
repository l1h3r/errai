use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

/// The raw bits of a reference.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct RawRef {
  number: u64,
  thread: u64,
}

impl RawRef {
  /// Returns the reference number value.
  #[inline]
  pub const fn number(&self) -> u64 {
    self.number
  }

  /// Returns the reference thread value.
  #[inline]
  pub const fn thread(&self) -> u64 {
    self.thread
  }
}

impl Debug for RawRef {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(self, f)
  }
}

impl Display for RawRef {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    todo!()
  }
}
