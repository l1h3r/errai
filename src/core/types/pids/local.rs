use ptab::Detached;
use ptab::config::DefaultParams;
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

  /// Bitmask for extracting the tag field.
  pub(crate) const TAG_MASK: usize = 1_usize.strict_shl(Self::TAG_BITS).strict_sub(1);

  /// Tag value identifying this as a PID type.
  pub(crate) const TAG_DATA: usize = (0x0 << Self::TAG_BITS) | 0x3;

  /// Bit width of the PID data fields (excluding tag).
  pub(crate) const PID_BITS: u32 = usize::BITS.strict_sub(Self::TAG_BITS);

  /// Bitmask for extracting the PID data fields.
  pub(crate) const PID_MASK: usize = 1_usize.strict_shl(Self::PID_BITS).strict_sub(1);

  /// The root process always gets the PID `0`.
  pub(crate) const ROOT_PROC: Self = Self::from_detached(Detached::from_bits(0));

  #[inline]
  pub(crate) const fn from_bits(bits: usize) -> Self {
    Self { bits }
  }

  #[inline]
  pub(crate) const fn into_bits(self) -> usize {
    self.bits
  }

  #[inline]
  const fn decompose(self) -> (u32, u32) {
    self.into_detached().decompose::<DefaultParams>()
  }

  #[inline]
  const fn from_detached(index: Detached) -> Self {
    debug_assert!(index.into_bits() & Self::PID_MASK == index.into_bits());

    let value: usize = index.into_bits() & Self::PID_MASK;
    let value: usize = (value << Self::TAG_BITS) | Self::TAG_DATA;

    Self::from_bits(value)
  }

  #[inline]
  const fn into_detached(self) -> Detached {
    debug_assert!(self.into_bits() & Self::TAG_MASK == Self::TAG_DATA);

    Detached::from_bits(self.into_bits() >> Self::TAG_BITS)
  }
}

impl Debug for LocalPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(self, f)
  }
}

impl Display for LocalPid {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    let (number, serial): (u32, u32) = self.decompose();
    write!(f, "#PID<0.{number}.{serial}>")
  }
}

// -----------------------------------------------------------------------------
// LocalPid <-> Detached
// -----------------------------------------------------------------------------

impl From<Detached> for LocalPid {
  #[inline]
  fn from(other: Detached) -> Self {
    Self::from_detached(other)
  }
}

impl From<LocalPid> for Detached {
  #[inline]
  fn from(other: LocalPid) -> Self {
    other.into_detached()
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use ptab::Detached;
  use ptab::config::CACHE_LINE_SLOTS;

  use crate::core::LocalPid;

  const DATA: Detached = Detached::from_bits(123 * CACHE_LINE_SLOTS);
  const BITS: usize = LocalPid::from_detached(DATA).into_bits();

  #[test]
  fn test_from_into_bits() {
    assert_eq!(BITS, LocalPid::from_bits(BITS).into_bits());
  }

  #[test]
  fn test_clone() {
    let src: LocalPid = LocalPid::from_bits(BITS);
    let dst: LocalPid = src.clone();

    assert_eq!(src, dst);
  }

  #[test]
  fn test_copy() {
    let src: LocalPid = LocalPid::from_bits(BITS);
    let dst: LocalPid = src;

    assert_eq!(src, dst);
  }

  #[test]
  fn test_display() {
    let src: LocalPid = LocalPid::from_bits(BITS);
    let fmt: String = format!("{src}");

    assert_eq!(fmt, "#PID<0.123.0>");
  }

  #[test]
  fn test_debug_equals_display() {
    let src: LocalPid = LocalPid::from_bits(BITS);
    let fmt: String = format!("{src}");

    assert_eq!(fmt, format!("{src:?}"));
  }
}
