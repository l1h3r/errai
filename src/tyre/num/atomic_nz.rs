use core::fmt::Debug;
use core::fmt::Formatter;
use core::fmt::Result;
use core::num::NonZeroU64;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::Ordering;

/// A non-zero integer type which can be safely shared between threads.
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
    // SAFETY: The value is `nonzero`.
    unsafe { Self::new_unchecked(value.get()) }
  }

  /// Loads a value from the atomic integer.
  #[inline]
  pub fn load(&self, order: Ordering) -> NonZeroU64 {
    // SAFETY: The value is always non-zero.
    unsafe { NonZeroU64::new_unchecked(self.inner.load(order)) }
  }

  /// Adds to the current value, returning the previous value.
  ///
  /// This operation wraps around on overflow.
  #[inline]
  pub fn fetch_add(&self, value: u64, ordering: Ordering) -> NonZeroU64 {
    'fetch_add: loop {
      let data: u64 = self.inner.fetch_add(value, ordering);

      if let Some(next) = NonZeroU64::new(data) {
        break 'fetch_add next;
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
