//! Local process identifier.
//!
//! # Bit Layout (64-bit)
//!
//! ```text
//! ┌─────────┬────────┬────────┬───────┐
//! │ Serial  │ Number │ Unused │ Tag   │
//! │ 32 bits │ 28 bits│ 0 bits │ 4 bits│
//! └─────────┴────────┴────────┴───────┘
//!           └─── PID_BITS (28/60) ────┘
//! ```
//!
//! - **Tag (4 bits)**: Type discriminator (0x3 for local PIDs)
//! - **Number (28 bits)**: Process table index
//! - **Serial (32 bits)**: Reuse counter to avoid aliasing

use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

/// Identifier uniquely naming a process on the local node.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LocalPid {
  bits: usize,
}

impl LocalPid {
  /// Bit width of the type tag field.
  pub(crate) const TAG_BITS: u32 = 4;

  /// Tag value identifying this as a PID type.
  pub(crate) const TAG_DATA: usize = (0x0 << Self::TAG_BITS) | 0x3;

  /// Bitmask for extracting the tag field.
  pub(crate) const TAG_MASK: usize = 0xF;

  /// Bit width of the PID data fields (excluding tag).
  pub(crate) const PID_BITS: u32 = usize::BITS - Self::TAG_BITS;

  /// Bit width of the process table index field.
  pub(crate) const NUMBER_BITS: u32 = 28;

  /// Bit width of the serial number field.
  pub(crate) const SERIAL_BITS: u32 = Self::PID_BITS - Self::NUMBER_BITS;

  /// Bitmask for extracting the PID data fields.
  pub(crate) const PID_MASK: usize = (1_usize << Self::PID_BITS) - 1;

  /// Bitmask for extracting the process table index field.
  pub(crate) const NUMBER_MASK: usize = (1_usize << Self::NUMBER_BITS) - 1;

  #[inline]
  pub(crate) const fn from_bits(bits: usize) -> Self {
    Self { bits }
  }

  #[inline]
  pub(crate) const fn into_bits(self) -> usize {
    self.bits
  }
}

impl Debug for LocalPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(self, f)
  }
}

impl Display for LocalPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    write!(f, "#PID<0.x.x>")
  }
}
