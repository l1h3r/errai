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

static GLOBAL_REF: CachePadded<LazyLock<AtomicU64>> = CachePadded::new(LazyLock::new(|| {
  AtomicU64::new(InternalRef::initialize(Runtime::time()))
}));

/// An internal reference.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct InternalRef {
  bits: [u32; 3],
}

impl InternalRef {
  pub(crate) const NUMBER_BITS: u32 = 18;
  pub(crate) const SERIAL_BITS: u32 = u32::BITS - Self::NUMBER_BITS;

  pub(crate) const NUMBER_MASK: u32 = (1 << Self::NUMBER_BITS) - 1;
  pub(crate) const SERIAL_MASK: u32 = ((1 << Self::SERIAL_BITS) - 1) << Self::NUMBER_BITS;

  /// Creates a new `InternalRef`.
  ///
  /// Note: References are currently implemented using a 64-bit global counter.
  ///
  /// Erlang/OTP uses a per-scheduler counter so that's something to consider...
  #[inline]
  pub(crate) fn new_global() -> Self {
    let global_id: u64 = GLOBAL_REF.fetch_add(1, Ordering::Relaxed);
    let thread_id: u32 = 0;

    Self {
      bits: Self::pack_bits(global_id, thread_id),
    }
  }

  /// Creates a new `InternalRef` from the given `bits`.
  #[inline]
  pub(crate) const fn from_bits(bits: [u32; 3]) -> Self {
    Self { bits }
  }

  /// Converts `self` into raw bits.
  #[inline]
  pub(crate) const fn into_bits(self) -> [u32; 3] {
    self.bits
  }

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
