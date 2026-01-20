use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::bifs;
use crate::core::ExternalPid;
use crate::core::ProcessId;

/// Identifier uniquely naming a process on the local node.
///
/// Internal PIDs are 64-bit tagged values that encode:
///
/// - **Index**: Process table slot (28 bits)
/// - **Serial**: Reuse counter to prevent PID collision (32 bits)
/// - **Tag**: Type tag for runtime type checking (4 bits)
///
/// # Format
///
/// PIDs display as `#PID<0.Number.Serial>` where `0` indicates the local node.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct InternalPid {
  bits: u64,
}

impl InternalPid {
  /// Bit width of the type tag field.
  pub(crate) const TAG_BITS: u32 = 4;

  /// Bit width of the PID data fields (excluding tag).
  pub(crate) const PID_BITS: u32 = u64::BITS - Self::TAG_BITS;

  /// Tag value identifying this as a PID type.
  pub(crate) const TAG_DATA: u64 = (0x0 << Self::TAG_BITS) | 0x3;

  /// Bitmask for extracting the tag field.
  pub(crate) const TAG_MASK: u64 = 0xF;

  /// Bit width of the process table index field.
  pub(crate) const NUMBER_BITS: u32 = 28;

  /// Bit width of the serial number field.
  pub(crate) const SERIAL_BITS: u32 = Self::PID_BITS - Self::NUMBER_BITS;

  /// Sentinel value representing an undefined or invalid PID.
  pub(crate) const UNDEFINED: Self = Self::from_bits(u64::MAX);

  /// Creates an internal PID from its raw encoded bits.
  ///
  /// This is used for deserialization or when reconstructing PIDs from
  /// stored data. The bits should include the tag, index, and serial fields.
  #[inline]
  pub const fn from_bits(bits: u64) -> Self {
    Self { bits }
  }

  /// Converts this PID into its raw encoded bits.
  ///
  /// This is used for serialization or when storing PIDs in compact form.
  #[inline]
  pub const fn into_bits(self) -> u64 {
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
    if let Some((number, serial)) = bifs::translate_pid(*self) {
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use hashbrown::HashSet;

  use crate::core::InternalPid;

  #[test]
  fn test_layout_constants() {
    assert_eq!(
      InternalPid::NUMBER_BITS + InternalPid::SERIAL_BITS,
      (8 * size_of::<InternalPid>() as u32) - InternalPid::TAG_BITS,
    );
  }

  #[test]
  fn test_undefined() {
    assert_eq!(InternalPid::UNDEFINED.into_bits(), u64::MAX);
  }

  #[test]
  fn test_from_bits_roundtrip() {
    let src: u64 = 0x1234_5678_9ABC_DEF0;
    let pid: InternalPid = InternalPid::from_bits(src);

    assert_eq!(pid.into_bits(), src);
  }

  #[test]
  fn test_clone() {
    let src: InternalPid = InternalPid::from_bits(123);
    let dst: InternalPid = src.clone();

    assert_eq!(src, dst);
  }

  #[test]
  fn test_copy() {
    let src: InternalPid = InternalPid::from_bits(123);
    let dst: InternalPid = src;

    assert_eq!(src, dst);
  }

  #[test]
  fn test_display() {
    let src: InternalPid = InternalPid::from_bits(123);
    let fmt: String = format!("{src}");

    assert_eq!(fmt, "#PID<0.x.x>");
  }

  #[test]
  fn test_debug_equals_display() {
    let src: InternalPid = InternalPid::from_bits(123);
    let fmt: String = format!("{src}");

    assert_eq!(fmt, format!("{src:?}"));
  }

  #[test]
  fn test_equality() {
    let a: InternalPid = InternalPid::from_bits(100);
    let b: InternalPid = InternalPid::from_bits(100);
    let c: InternalPid = InternalPid::from_bits(200);

    assert_eq!(a, b);
    assert_ne!(a, c);
  }

  #[test]
  fn test_ordering() {
    let a: InternalPid = InternalPid::from_bits(100);
    let b: InternalPid = InternalPid::from_bits(200);
    let c: InternalPid = InternalPid::from_bits(300);

    assert!(a < b);
    assert!(b < c);
    assert!(a < c);
  }

  #[test]
  fn test_hash() {
    let mut set: HashSet<InternalPid> = HashSet::new();

    set.insert(InternalPid::from_bits(1));
    set.insert(InternalPid::from_bits(2));
    set.insert(InternalPid::from_bits(1));

    assert_eq!(set.len(), 2);
  }
}
