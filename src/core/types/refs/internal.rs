use crossbeam_utils::CachePadded;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use std::sync::LazyLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::erts::Runtime;

/// Global reference counter initialized with timestamp-based seed.
///
/// This counter ensures uniqueness across the node's lifetime by combining
/// timestamp data with a monotonic sequence.
static GLOBAL_REF: CachePadded<LazyLock<AtomicU64>> = CachePadded::new(LazyLock::new(|| {
  AtomicU64::new(InternalRef::initialize(Runtime::time()))
}));

/// Reference uniquely identifying runtime objects on the local node.
///
/// Internal references are 96-bit values (3xu32) generated from a monotonic
/// global counter. The counter is initialized with timestamp data to provide
/// uniqueness across runtime restarts.
///
/// # Format
///
/// References display as `#Ref<0.X.Y.Z>` where:
///
/// - `0`: Node indicator (local node)
/// - `X`, `Y`, `Z`: 32-bit components from the counter
///
/// # Thread Safety
///
/// References are generated atomically from a global counter with relaxed
/// ordering, providing uniqueness without synchronization overhead.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct InternalRef {
  bits: [u32; 3],
}

impl InternalRef {
  /// Bit width for the number field in reference encoding.
  pub(crate) const NUMBER_BITS: u32 = 18;

  /// Bit width for the serial field in reference encoding.
  pub(crate) const SERIAL_BITS: u32 = u32::BITS - Self::NUMBER_BITS;

  /// Bitmask for extracting the number field.
  pub(crate) const NUMBER_MASK: u32 = (1 << Self::NUMBER_BITS) - 1;

  /// Bitmask for extracting the serial field.
  pub(crate) const SERIAL_MASK: u32 = ((1 << Self::SERIAL_BITS) - 1) << Self::NUMBER_BITS;

  /// Creates a new internal reference from the global counter.
  ///
  /// This is the primary way to generate new references. Each call increments
  /// the global counter and returns a unique reference.
  #[inline]
  pub(crate) fn new_global() -> Self {
    let global_id: u64 = GLOBAL_REF.fetch_add(1, Ordering::Relaxed);
    let thread_id: u32 = 0;

    Self::from_bits(Self::pack_bits(global_id, thread_id))
  }

  /// Creates an internal reference from its raw bits.
  ///
  /// This is used for deserialization or when reconstructing references
  /// from stored data.
  #[inline]
  pub(crate) const fn from_bits(bits: [u32; 3]) -> Self {
    Self { bits }
  }

  /// Converts this reference into its raw bits.
  ///
  /// This is used for serialization or when storing references in compact form.
  #[inline]
  pub(crate) const fn into_bits(self) -> [u32; 3] {
    self.bits
  }

  /// Packs global counter and thread ID into reference bit layout.
  #[inline]
  fn pack_bits(global_id: u64, thread_id: u32) -> [u32; 3] {
    debug_assert_eq!(thread_id, thread_id & Self::NUMBER_MASK);
    let mut value: [u32; 3] = [0; 3];
    value[0] |= (global_id & Self::NUMBER_MASK as u64) as u32;
    value[1] |= (global_id & Self::SERIAL_MASK as u64) as u32;
    value[1] |= thread_id & Self::NUMBER_MASK;
    value[2] |= (global_id >> u32::BITS) as u32;
    value
  }

  #[inline]
  const fn initialize(timestamp: Duration) -> u64 {
    let mut data: u64 = 0;
    data |= timestamp.as_secs();
    data |= (timestamp.subsec_micros() as u64) << 32;
    data = data.wrapping_mul(268438039); // TODO: Magic Erlang Value
    data = data.wrapping_add(timestamp.subsec_micros() as u64);
    data
  }
}

impl Debug for InternalRef {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Display::fmt(self, f)
  }
}

impl Display for InternalRef {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    write!(
      f,
      "#Ref<0.{}.{}.{}>",
      self.bits[2], self.bits[1], self.bits[0],
    )
  }
}
