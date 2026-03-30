use std::cell::Cell;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::sync::LazyLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::thread::AccessError;
use std::time::Duration;

use crate::core::fatal;
use crate::utils::hash;
use crate::utils::thread;
use crate::utils::time;

// -----------------------------------------------------------------------------
// Local Ref
// -----------------------------------------------------------------------------

/// Reference uniquely identifying runtime objects on the local node.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct LocalRef {
  bits: [u32; 3],
}

impl LocalRef {
  /// Bit width for the low number bits.
  pub(crate) const LOW_BITS: u32 = 18;

  /// Bit width for the mid number bits.
  pub(crate) const MID_BITS: u32 = u32::BITS.strict_sub(Self::LOW_BITS);

  /// Bitmask for extracting the low number bits.
  pub(crate) const LOW_MASK: u32 = 1_u32.strict_shl(Self::LOW_BITS).strict_sub(1);

  /// Bitmask for extracting the mid number bits.
  ///
  /// Note: This mask is pre-shifted.
  pub(crate) const MID_MASK: u32 = 1_u32
    .strict_shl(Self::MID_BITS)
    .strict_sub(1)
    .strict_shl(Self::LOW_BITS);

  /// Creates a new local reference.
  #[expect(clippy::new_without_default, reason = "possibly confusing")]
  #[inline]
  pub fn new() -> Self {
    let thread: u32;
    let number: u64;

    if let Ok(thread_id) = thread::ThreadId::current() {
      thread = thread_id.as_u32().get();
      number = next_thread_ref();
    } else {
      thread = 0;
      number = GLOBAL_REF.fetch_add(1, Ordering::Relaxed);
    }

    Self::from_bits(Self::pack_bits(thread, number))
  }

  /// Creates a local reference from its raw bits.
  #[inline]
  pub(crate) const fn from_bits(bits: [u32; 3]) -> Self {
    Self { bits }
  }

  /// Converts this reference into its raw bits.
  #[inline]
  pub(crate) const fn into_bits(self) -> [u32; 3] {
    self.bits
  }

  /// Returns the thread ID component.
  #[inline]
  pub(crate) const fn thread(&self) -> u32 {
    self.bits[1] & Self::LOW_MASK
  }

  /// Returns the 64-bit counter component.
  #[inline]
  pub(crate) const fn number(&self) -> u64 {
    ((self.bits[0] & Self::LOW_MASK) as u64)
      | ((self.bits[1] & Self::MID_MASK) as u64)
      | ((self.bits[2] as u64) << u32::BITS)
  }

  /// Packs thread ID and counter into reference bit layout.
  #[inline]
  fn pack_bits(thread: u32, number: u64) -> [u32; 3] {
    debug_assert_eq!(thread, thread & Self::LOW_MASK);
    let mut value: [u32; 3] = [0; 3];
    value[0] |= (number & Self::LOW_MASK as u64) as u32;
    value[1] |= (number & Self::MID_MASK as u64) as u32;
    value[1] |= thread & Self::LOW_MASK;
    value[2] |= (number >> u32::BITS) as u32;
    value
  }
}

impl Debug for LocalRef {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    Display::fmt(self, f)
  }
}

impl Display for LocalRef {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    let a: u32 = self.bits[0];
    let b: u32 = self.bits[1];
    let c: u32 = self.bits[2];

    write!(f, "#Ref<0.{c}.{b}.{a}>")
  }
}

// -----------------------------------------------------------------------------
// Global Ref Counter
// -----------------------------------------------------------------------------

static GLOBAL_REF: LazyLock<AtomicU64> =
  LazyLock::new(|| AtomicU64::new(init_thread_ref(time::unix())));

// -----------------------------------------------------------------------------
// Thread-local Ref Counters
// -----------------------------------------------------------------------------

thread_local! {
  static THREAD_REF: Cell<u64> = Cell::new(init_thread_ref(time::unix()));
}

/// This nice init routine was taken from Erlang/OTP and helps
/// prevent counter collision across runtime restarts.
#[inline]
fn init_thread_ref(timestamp: Duration) -> u64 {
  let mut data: u64 = 0;
  data |= timestamp.as_secs();
  data |= (timestamp.subsec_micros() as u64) << 32;
  data = data.wrapping_mul(hash::ERL_FUNNY_NUMBER7);
  data = data.wrapping_add(timestamp.subsec_micros() as u64);
  data
}

#[inline]
fn read_thread_ref() -> Result<u64, AccessError> {
  THREAD_REF.try_with(|cell| {
    let data: u64 = cell.get();
    cell.set(data.wrapping_add(1));
    data
  })
}

#[inline]
fn next_thread_ref() -> u64 {
  match read_thread_ref() {
    Ok(number) => number,
    Err(error) => fatal!(error),
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::collections::HashSet;
  use std::sync::Arc;
  use std::sync::Mutex;
  use std::thread;

  use crate::core::LocalRef;

  const BITS: [u32; 3] = [1, 2, 3];

  #[test]
  fn test_new() {
    let ref1: LocalRef = LocalRef::new();
    let ref2: LocalRef = LocalRef::new();

    assert_ne!(ref1, ref2);
  }

  #[test]
  fn test_from_into_bits() {
    assert_eq!(BITS, LocalRef::from_bits(BITS).into_bits());
  }

  #[test]
  fn test_extract_thread_and_number() {
    let thread: u32 = 123;
    let number: u64 = 456;
    let packed: [u32; 3] = LocalRef::pack_bits(thread, number);
    let target: LocalRef = LocalRef::from_bits(packed);

    assert_eq!(target.thread(), thread);
    assert_eq!(target.number(), number);
  }

  #[test]
  fn test_clone() {
    let src: LocalRef = LocalRef::from_bits(BITS);
    let dst: LocalRef = src.clone();

    assert_eq!(src, dst);
  }

  #[test]
  fn test_copy() {
    let src: LocalRef = LocalRef::from_bits(BITS);
    let dst: LocalRef = src;

    assert_eq!(src, dst);
  }

  #[test]
  fn test_display() {
    let src: LocalRef = LocalRef::from_bits(BITS);
    let fmt: String = format!("{src}");

    assert_eq!(fmt, "#Ref<0.3.2.1>");
  }

  #[test]
  fn test_debug_equals_display() {
    let src: LocalRef = LocalRef::from_bits(BITS);
    let fmt: String = format!("{src}");

    assert_eq!(fmt, format!("{src:?}"));
  }

  #[test]
  fn test_concurrent_new_10_threads() {
    const THREADS: usize = 10;
    const ACTIONS: usize = 100;

    let refs: Arc<_> = Arc::new(Mutex::new(HashSet::new()));

    let handles: Vec<_> = (0..THREADS)
      .map(|_| {
        let refs: Arc<_> = refs.clone();

        thread::spawn(move || {
          for _ in 0..ACTIONS {
            refs.lock().unwrap().insert(LocalRef::new());
          }
        })
      })
      .collect();

    for handle in handles {
      handle.join().unwrap();
    }

    assert_eq!(refs.lock().unwrap().len(), THREADS.strict_mul(ACTIONS));
  }
}
