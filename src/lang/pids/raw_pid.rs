use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

/// The raw bits of a process identifier.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RawPid {
  bits: u64,
}

impl RawPid {
  /// Returns the PID number value.
  #[inline]
  pub const fn number(self) -> u32 {
    self.bits as u32
  }

  /// Returns the PID serial value.
  #[inline]
  pub const fn serial(self) -> u32 {
    (self.bits >> u32::BITS) as u32
  }
}

impl Debug for RawPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(self, f)
  }
}

impl Display for RawPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    write!(f, "#PID<0.{}.{}>", self.number(), self.serial())
  }
}
