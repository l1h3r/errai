use core::fmt::Debug;
use core::fmt::Formatter;
use core::fmt::Pointer;
use core::fmt::Result as FmtResult;
use core::panic::RefUnwindSafe;
use core::sync::atomic::AtomicPtr;
use core::sync::atomic::Ordering;

use crate::tyre::ptr::TaggedPtr;

/// A raw pointer with a tag value, which can be safely shared between threads.
///
/// This type has the same size and bit validity as a [`AtomicPtr<T>`].
#[repr(transparent)]
pub struct AtomicTaggedPtr<T> {
  inner: AtomicPtr<T>,
}

impl<T> AtomicTaggedPtr<T> {
  // ---------------------------------------------------------------------------
  // Pointer Tag API
  // ---------------------------------------------------------------------------

  /// The total number of bits available to store a tag value.
  pub const BITS: u32 = TaggedPtr::<T>::BITS;

  /// A bitmask covering the tag value bits of a pointer.
  pub const TAG_MASK: usize = TaggedPtr::<T>::TAG_MASK;

  /// A bitmask covering the address bits of a pointer.
  pub const PTR_MASK: usize = TaggedPtr::<T>::PTR_MASK;

  // ---------------------------------------------------------------------------
  // Core Pointer API
  // ---------------------------------------------------------------------------

  /// Creates a null mutable atomic tagged pointer.
  ///
  /// # Panics
  ///
  /// This method will panic if `*mut T` does not have sufficient alignment to
  /// hold a tag value.
  #[must_use]
  #[inline]
  pub const fn null() -> Self {
    Self::from_tagged(TaggedPtr::null())
  }

  /// Creates a new `AtomicTaggedPtr` if `ptr` has sufficient alignment to hold
  /// a tag value.
  #[must_use]
  #[inline]
  pub const fn new(ptr: *mut T) -> Option<AtomicTaggedPtr<T>> {
    if let Some(tagged) = TaggedPtr::new(ptr) {
      Some(Self::from_tagged(tagged))
    } else {
      None
    }
  }

  /// Creates a new `AtomicTaggedPtr`.
  ///
  /// # Safety
  ///
  /// `ptr` must have sufficient alignment to hold a tag value.
  #[must_use]
  #[inline]
  pub const unsafe fn new_unchecked(ptr: *mut T) -> Self {
    // SAFETY: This is guaranteed to be safe by the caller.
    Self::from_tagged(unsafe { TaggedPtr::new_unchecked(ptr) })
  }

  /// Creates a new `AtomicTaggedPtr` from `ptr`.
  #[inline]
  pub const fn from_tagged(ptr: TaggedPtr<T>) -> AtomicTaggedPtr<T> {
    Self {
      inner: AtomicPtr::new(ptr.as_ptr_tagged()),
    }
  }

  /// Consumes the atomic and returns the contained value (**untagged**).
  ///
  /// This is safe because passing `self` by value guarantees that no other
  /// threads are concurrently accessing the atomic data.
  #[must_use]
  #[inline]
  pub const fn into_inner(self) -> *mut T {
    self.inner.into_inner()
  }

  /// Consumes the atomic and returns the contained value.
  ///
  /// This is safe because passing `self` by value guarantees that no other
  /// threads are concurrently accessing the atomic data.
  #[must_use]
  #[inline]
  pub const fn into_tagged(self) -> TaggedPtr<T> {
    // SAFETY: `self` is already known to have valid alignment.
    unsafe { TaggedPtr::new_unchecked(self.into_inner()) }
  }

  // ---------------------------------------------------------------------------
  // Atomic Pointer API
  // ---------------------------------------------------------------------------

  /// Loads a value from the pointer.
  ///
  /// # Panics
  ///
  /// Panics if `order` is [`Release`][Ordering::Release] or [`AcqRel`][Ordering::AcqRel].
  #[inline]
  pub fn load(&self, order: Ordering) -> TaggedPtr<T> {
    // SAFETY: `self` is already known to have valid alignment.
    unsafe { TaggedPtr::new_unchecked(self.inner.load(order)) }
  }

  /// Stores a value into the pointer.
  ///
  /// # Panics
  ///
  /// Panics if `order` is [`Acquire`][Ordering::Acquire] or [`AcqRel`][Ordering::AcqRel].
  #[inline]
  pub fn store(&self, ptr: TaggedPtr<T>, order: Ordering) {
    self.inner.store(ptr.as_ptr_tagged(), order);
  }

  /// Stores a value into the pointer, returning the previous value.
  #[inline]
  pub fn swap(&self, ptr: TaggedPtr<T>, order: Ordering) -> TaggedPtr<T> {
    // SAFETY: `self` is already known to have valid alignment.
    unsafe { TaggedPtr::new_unchecked(self.inner.swap(ptr.as_ptr_tagged(), order)) }
  }

  /// Stores a value into the pointer if the current value is the same as the
  /// `current` value.
  ///
  /// The return value is a result indicating whether the new value was written
  /// and containing the previous value. On success this value is guaranteed to
  /// be equal to `current`.
  #[inline]
  pub fn compare_exchange(
    &self,
    current: TaggedPtr<T>,
    new: TaggedPtr<T>,
    success: Ordering,
    failure: Ordering,
  ) -> Result<TaggedPtr<T>, TaggedPtr<T>> {
    let ptr_a: *mut T = current.as_ptr_tagged();
    let ptr_b: *mut T = new.as_ptr_tagged();

    // SAFETY: `TaggedPtr<T>` is already known to have valid alignment.
    self
      .inner
      .compare_exchange(ptr_a, ptr_b, success, failure)
      .map(|ptr| unsafe { TaggedPtr::new_unchecked(ptr) })
      .map_err(|ptr| unsafe { TaggedPtr::new_unchecked(ptr) })
  }

  /// Stores a value into the pointer if the current value is the same as the
  /// `current` value.
  ///
  /// Unlike [`compare_exchange`][Self::compare_exchange], this function is
  /// allowed to spuriously fail even when the comparison succeeds, which can
  /// result in more efficient code on some platforms. The return value is a
  /// result indicating whether the new value was written and containing the
  /// previous value.
  #[inline]
  pub fn compare_exchange_weak(
    &self,
    current: TaggedPtr<T>,
    new: TaggedPtr<T>,
    success: Ordering,
    failure: Ordering,
  ) -> Result<TaggedPtr<T>, TaggedPtr<T>> {
    let ptr_a: *mut T = current.as_ptr_tagged();
    let ptr_b: *mut T = new.as_ptr_tagged();

    // SAFETY: `TaggedPtr<T>` is already known to have valid alignment.
    self
      .inner
      .compare_exchange_weak(ptr_a, ptr_b, success, failure)
      .map(|ptr| unsafe { TaggedPtr::new_unchecked(ptr) })
      .map_err(|ptr| unsafe { TaggedPtr::new_unchecked(ptr) })
  }

  /// Fetches the value, and applies a function to it that returns an optional
  /// new value. Returns a `Result` of `Ok(previous_value)` if the function
  /// returned `Some(_)`, else `Err(previous_value)`.
  #[inline]
  pub fn fetch_update<F>(
    &self,
    store_order: Ordering,
    fetch_order: Ordering,
    mut f: F,
  ) -> Result<TaggedPtr<T>, TaggedPtr<T>>
  where
    F: FnMut(TaggedPtr<T>) -> Option<TaggedPtr<T>>,
  {
    let intercept = |ptr: *mut T| -> Option<*mut T> {
      // SAFETY: `ptr` is already known to have valid alignment.
      let tag: TaggedPtr<T> = unsafe { TaggedPtr::new_unchecked(ptr) };
      let new: Option<TaggedPtr<T>> = f(tag);

      new.map(TaggedPtr::as_ptr_tagged)
    };

    // SAFETY: `TaggedPtr<T>` is already known to have valid alignment.
    self
      .inner
      .fetch_update(store_order, fetch_order, intercept)
      .map(|ptr| unsafe { TaggedPtr::new_unchecked(ptr) })
      .map_err(|ptr| unsafe { TaggedPtr::new_unchecked(ptr) })
  }

  /// Fetches the value, applies a function to it that it return a new value.
  /// The new value is stored and the old value is returned.
  #[inline]
  pub fn update(
    &self,
    store_order: Ordering,
    fetch_order: Ordering,
    mut f: impl FnMut(TaggedPtr<T>) -> TaggedPtr<T>,
  ) -> TaggedPtr<T> {
    let mut prev: TaggedPtr<T> = self.load(fetch_order);

    'update: loop {
      match self.compare_exchange_weak(prev, f(prev), store_order, fetch_order) {
        Ok(value) => break 'update value,
        Err(next) => prev = next,
      }
    }
  }
}

// SAFETY: `AtomicTaggedPtr<T>` is safe to transfer across
//         thread boundaries because all accesses are atomic.
unsafe impl<T> Send for AtomicTaggedPtr<T> {}

// SAFETY: `AtomicTaggedPtr<T>` is safe to share between
//         threads because all accesses are atomic.
unsafe impl<T> Sync for AtomicTaggedPtr<T> {}

impl<T> Debug for AtomicTaggedPtr<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    Debug::fmt(&self.load(Ordering::Relaxed), f)
  }
}

impl<T> Pointer for AtomicTaggedPtr<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    Pointer::fmt(&self.load(Ordering::Relaxed), f)
  }
}

impl<T> Default for AtomicTaggedPtr<T> {
  #[inline]
  fn default() -> Self {
    Self::null()
  }
}

impl<T> RefUnwindSafe for AtomicTaggedPtr<T> {}

impl<T> From<TaggedPtr<T>> for AtomicTaggedPtr<T> {
  /// Converts a `TaggedPtr<T>` into an `AtomicTaggedPtr<T>`.
  #[inline]
  fn from(other: TaggedPtr<T>) -> Self {
    AtomicTaggedPtr::from_tagged(other)
  }
}
