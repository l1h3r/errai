use core::fmt::Debug;
use core::fmt::Formatter;
use core::fmt::Result;
use core::num::NonZeroU64;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;

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
  pub fn new() -> Self {
    Self::from_nonzero(Self::NZ_ONE)
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
    Self::new()
  }
}

impl From<NonZeroU64> for AtomicNzU64 {
  #[inline]
  fn from(other: NonZeroU64) -> Self {
    Self::from_nonzero(other)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_fetch_add_wrapping() {
    let atomic = AtomicNzU64::from_nonzero(NonZeroU64::new(1).unwrap());

    // This should handle wrapping correctly
    let previous: NonZeroU64 = atomic.fetch_add(u64::MAX, Ordering::Relaxed);
    assert_eq!(previous.get(), 1);

    // Atomic should never be zero
    let current: NonZeroU64 = atomic.load(Ordering::Relaxed);
    assert_ne!(current.get(), 0, "Atomic value wrapped to zero - UB!");
  }

  #[test]
  fn test_multiple_wraps() {
    let atomic = AtomicNzU64::new();

    for _ in 0..1000 {
      atomic.fetch_add(u64::MAX / 100, Ordering::Relaxed);
      assert_ne!(atomic.load(Ordering::Relaxed).get(), 0);
    }
  }
}
