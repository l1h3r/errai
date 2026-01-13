use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::bifs::translate_pid;
use crate::lang::ExternalPid;
use crate::lang::ProcessId;

/// An internal process identifier.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct InternalPid {
  bits: u64,
}

impl InternalPid {
  pub(crate) const TAG_BITS: u32 = 4;
  pub(crate) const PID_BITS: u32 = u32::BITS - Self::TAG_BITS;

  pub(crate) const TAG_DATA: u64 = (0x0 << Self::TAG_BITS) | 0x3;
  pub(crate) const TAG_MASK: u64 = 0xF;

  pub(crate) const NUMBER_BITS: u32 = 28;
  pub(crate) const SERIAL_BITS: u32 = Self::PID_BITS - Self::NUMBER_BITS;

  /// Creates a new `InternalPid` from the given `bits`.
  #[inline]
  pub(crate) const fn from_bits(bits: u64) -> Self {
    Self { bits }
  }

  /// Converts `self` into raw bits.
  #[inline]
  pub(crate) const fn into_bits(self) -> u64 {
    self.bits
  }
}

impl Debug for InternalPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(self, f)
  }
}

impl Display for InternalPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    // For internal PIDs, we use `0` as the channel number.
    // For external PIDs (formatted elsewhere), we use the node name index.
    //
    // Note: We need access to the readonly data of the process table to
    //       understand the bit transformations required to format the PID.
    if let Some((number, serial)) = translate_pid(*self) {
      write!(f, "#PID<0.{}.{}>", number, serial)
    } else {
      write!(f, "#PID<0.x.x>")
    }
  }
}

impl ProcessId for InternalPid {
  const DISTRIBUTED: bool = false;

  #[inline]
  fn into_internal(self) -> InternalPid {
    self
  }

  #[inline]
  fn into_external(self) -> Option<ExternalPid> {
    None
  }
}
