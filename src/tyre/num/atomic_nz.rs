use core::fmt::Debug;
use core::fmt::Formatter;
use core::fmt::Result;
use core::num::NonZeroU64;

use crate::loom::sync::atomic::AtomicU64;
use crate::loom::sync::atomic::Ordering;

/// A non-zero integer type which can be safely shared between threads.
///
/// This type has the same size and bit validity as a [`AtomicU64`].
#[repr(transparent)]
pub struct AtomicNzU64 {
  inner: AtomicU64,
}

impl AtomicNzU64 {
  // SAFETY: The value is `1`.
  const NZ_ONE: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(1) };

  /// Creates a new `AtomicNzU64`.
  #[inline]
  pub fn new(value: u64) -> Option<Self> {
    NonZeroU64::new(value).map(Self::from_nonzero)
  }

  /// Creates a new `AtomicNzU64` without checking whether the value is non-zero.
  ///
  /// # Safety
  ///
  /// The value must not be zero.
  #[inline]
  pub unsafe fn new_unchecked(value: u64) -> Self {
    Self {
      inner: AtomicU64::new(value),
    }
  }

  /// Creates a new `AtomicNzU64` from a non-zero value.
  #[inline]
  pub fn from_nonzero(value: NonZeroU64) -> Self {
    // SAFETY: The value is non-zero.
    unsafe { Self::new_unchecked(value.get()) }
  }

  /// Loads a value from the atomic integer.
  #[inline]
  pub fn load(&self, order: Ordering) -> NonZeroU64 {
    // SAFETY: `self` is already known to be non-zero.
    unsafe { NonZeroU64::new_unchecked(self.inner.load(order)) }
  }

  /// Adds to the current value, returning the previous value.
  ///
  /// This operation wraps around on overflow.
  #[inline]
  pub fn fetch_add(&self, value: u64, ordering: Ordering) -> NonZeroU64 {
    'fetch_add: loop {
      let current: u64 = self.inner.load(Ordering::Relaxed);
      let updated: u64 = current.wrapping_add(value);

      // Skip zero by adding 1 more
      let updated: u64 = if updated == 0 { 1 } else { updated };

      match self
        .inner
        .compare_exchange_weak(current, updated, ordering, Ordering::Relaxed)
      {
        Ok(prev) => {
          // SAFETY: We just loaded `current` and verified it's non-zero via invariant
          break 'fetch_add unsafe { NonZeroU64::new_unchecked(prev) };
        }
        Err(_) => continue,
      }
    }
  }
}

impl Debug for AtomicNzU64 {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Debug::fmt(&self.inner, f)
  }
}

impl Default for AtomicNzU64 {
  #[inline]
  fn default() -> Self {
    Self::from_nonzero(Self::NZ_ONE)
  }
}

impl From<NonZeroU64> for AtomicNzU64 {
  #[inline]
  fn from(other: NonZeroU64) -> Self {
    Self::from_nonzero(other)
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::num::NonZeroU64;

  use crate::loom::sync::atomic::Ordering;
  use crate::tyre::num::AtomicNzU64;

  #[test]
  fn test_new_rejects_zero() {
    assert!(AtomicNzU64::new(0).is_none());
  }

  #[test]
  fn test_new_accepts_nonzero() {
    assert!(AtomicNzU64::new(1).is_some());
    assert!(AtomicNzU64::new(123).is_some());
    assert!(AtomicNzU64::new(u64::MAX).is_some());
  }

  #[test]
  fn test_from_nonzero() {
    let source: NonZeroU64 = NonZeroU64::new(123).unwrap();
    let atomic: AtomicNzU64 = AtomicNzU64::from_nonzero(source);

    assert_eq!(atomic.load(Ordering::Relaxed).get(), 123);
  }

  #[test]
  fn test_load_returns_nonzero() {
    let atomic: AtomicNzU64 = AtomicNzU64::new(456).unwrap();
    let loaded: NonZeroU64 = atomic.load(Ordering::Relaxed);

    assert_eq!(loaded.get(), 456);
  }

  #[test]
  fn test_default_is_one() {
    let atomic: AtomicNzU64 = AtomicNzU64::default();
    let loaded: NonZeroU64 = atomic.load(Ordering::Relaxed);

    assert_eq!(loaded.get(), 1);
  }

  #[test]
  fn test_fetch_add_simple() {
    let atomic: AtomicNzU64 = AtomicNzU64::new(10).unwrap();

    assert_eq!(atomic.fetch_add(5, Ordering::Relaxed).get(), 10);
    assert_eq!(atomic.load(Ordering::Relaxed).get(), 15);
  }

  #[test]
  fn test_fetch_add_wrapping_to_one() {
    let atomic: AtomicNzU64 = AtomicNzU64::new(1).unwrap();

    assert_eq!(atomic.fetch_add(u64::MAX, Ordering::Relaxed).get(), 1);
    assert_eq!(atomic.load(Ordering::Relaxed).get(), 1);
  }

  #[test]
  fn test_fetch_add_near_max() {
    let atomic: AtomicNzU64 = AtomicNzU64::new(u64::MAX - 5).unwrap();

    assert_eq!(atomic.fetch_add(10, Ordering::Relaxed).get(), u64::MAX - 5);
    assert_ne!(atomic.load(Ordering::Relaxed).get(), 0);
  }

  #[test]
  fn test_fetch_add_multiple_wraps() {
    let atomic: AtomicNzU64 = AtomicNzU64::new(u64::MAX - 1).unwrap();

    for _ in 0..1000 {
      assert_ne!(atomic.fetch_add(u64::MAX / 10, Ordering::Relaxed).get(), 0);
      assert_ne!(atomic.load(Ordering::Relaxed).get(), 0);
    }
  }

  #[test]
  fn test_fetch_add_preserves_nonzero() {
    let atomic: AtomicNzU64 = AtomicNzU64::new(100).unwrap();

    for index in 1..100 {
      let _pass: NonZeroU64 = atomic.fetch_add(index, Ordering::Relaxed);
      let value: NonZeroU64 = atomic.load(Ordering::Relaxed);

      assert_ne!(value.get(), 0, "Value became zero at iteration {index}");
    }
  }

  #[test]
  fn test_debug() {
    let atomic: AtomicNzU64 = AtomicNzU64::new(789).unwrap();
    let format: String = format!("{:?}", atomic);

    assert!(format.contains("789"));
  }

  #[test]
  fn test_from() {
    let source: NonZeroU64 = NonZeroU64::new(321).unwrap();
    let atomic: AtomicNzU64 = source.into();

    assert_eq!(atomic.load(Ordering::Relaxed).get(), 321);
  }
}
