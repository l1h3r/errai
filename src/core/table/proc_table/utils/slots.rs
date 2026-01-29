use std::alloc::Layout;
use std::alloc::alloc;
use std::alloc::dealloc;
use std::alloc::handle_alloc_error;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::num::NonZeroUsize;
use std::ptr::NonNull;
use std::slice;

use crate::core::fatal;
use crate::core::table::proc_table::Index;
use crate::core::table::proc_table::Params;

#[repr(C)]
pub(crate) struct Slots<T> {
  ptr: NonNull<T>,
  len: NonZeroUsize,
}

impl<T> Slots<T> {
  /// Constructs a new slot array with uninitialized contents.
  #[inline]
  pub(crate) fn new_uninit(len: NonZeroUsize) -> Slots<MaybeUninit<T>> {
    let layout: Layout = Self::layout(len);

    // SAFETY: Layout is guaranteed to have a non-zero size.
    let target: *mut u8 = unsafe { alloc(layout) };

    let Some(nonnull) = NonNull::new(target) else {
      handle_alloc_error(layout);
    };

    Slots {
      ptr: nonnull.cast(),
      len,
    }
  }

  /// Returns a raw pointer to the list's buffer.
  #[inline]
  pub(crate) const fn as_ptr(&self) -> *const T {
    self.ptr.as_ptr()
  }

  /// Returns an unsafe mutable pointer to the list's buffer.
  #[inline]
  pub(crate) const fn as_mut_ptr(&mut self) -> *mut T {
    self.ptr.as_ptr()
  }

  /// Returns the total number of elements the list can hold.
  #[inline]
  pub(crate) const fn len(&self) -> NonZeroUsize {
    self.len
  }

  /// Extracts a slice containing the entire array.
  #[inline]
  pub(crate) const fn as_slice(&self) -> &[T] {
    unsafe { slice::from_raw_parts(self.as_ptr(), self.len.get()) }
  }

  /// Extracts a mutable slice of the entire array.
  #[inline]
  pub(crate) const fn as_mut_slice(&mut self) -> &mut [T] {
    unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len.get()) }
  }

  /// Returns a reference to the entry at `index`, or `None` if out of bounds.
  #[inline]
  pub(crate) const fn get<U>(&self, index: Index<'_, U>) -> &T {
    // SAFETY: The `Index` type is known to be in bounds.
    unsafe { self.get_unchecked(index.get()) }
  }

  /// Returns a reference to the entry at `index`, without doing bounds checking.
  ///
  /// # Safety
  ///
  /// Calling this method with an out-of-bounds index is *undefined behavior*
  /// even if the resulting reference is not used.
  #[inline]
  pub(crate) const unsafe fn get_unchecked(&self, index: usize) -> &T {
    debug_assert!(
      index < self.len.get(),
      "Slots::get_unchecked requires that the index is in bounds",
    );

    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.ptr.add(index).as_ref() }
  }

  #[inline]
  fn layout(len: NonZeroUsize) -> Layout {
    // This calculation is not really required (`Layout` would do it)
    // but we keep it here to stay in sync with `Params::new`.
    let mem_bytes: usize = len.get().strict_mul(size_of::<T>());
    let mem_align: usize = mem_bytes.next_multiple_of(Params::CACHE_LINE);

    match Layout::from_size_align(mem_align, Params::CACHE_LINE) {
      Ok(layout) if layout.size() != 0 => layout,
      Ok(_) => fatal!("invalid table layout"),
      Err(error) => fatal!(error),
    }
  }
}

impl<T> Slots<MaybeUninit<T>> {
  /// Converts `self` into an initialized array.
  ///
  /// # Safety
  ///
  /// This function has the same safety requirements as [`MaybeUninit::assume_init`].
  /// It is up to the caller to guarantee that all `MaybeUninit<T>` entries are
  /// really in an initialized state.
  #[inline]
  pub(crate) unsafe fn assume_init(self) -> Slots<T> {
    let len: NonZeroUsize = self.len;
    let ptr: NonNull<T> = ManuallyDrop::new(self).ptr.cast();

    Slots { ptr, len }
  }
}

impl<T> Drop for Slots<T> {
  fn drop(&mut self) {
    let target: *mut u8 = self.as_mut_ptr().cast();
    let layout: Layout = Self::layout(self.len);

    // SAFETY: The pointer was allocated by the global allocator using this exact
    //         layout. We're the sole owner at drop time, so deallocation is safe.
    unsafe { dealloc(target, layout) }
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use crossbeam_epoch::Atomic;
  use std::sync::atomic::AtomicUsize;

  use super::*;

  #[inline]
  fn create<T>(len: usize) -> Slots<T> {
    let capacity: NonZeroUsize = NonZeroUsize::new(len).unwrap();
    let mut this: Slots<MaybeUninit<T>> = Slots::new_uninit(capacity);

    unsafe {
      this.as_mut_ptr().write_bytes(0, len);
      this.assume_init()
    }
  }

  #[test]
  fn test_alignment() {
    for shift in Params::MIN_ENTRIES_SHIFT..=Params::MAX_ENTRIES_SHIFT {
      let concrete_array: Slots<Atomic<u64>> = create(1 << shift);
      let abstract_array: Slots<AtomicUsize> = create(1 << shift);

      assert_eq!(concrete_array.as_ptr().addr() % Params::CACHE_LINE, 0);
      assert_eq!(abstract_array.as_ptr().addr() % Params::CACHE_LINE, 0);
    }
  }

  #[test]
  fn test_length() {
    for shift in Params::MIN_ENTRIES_SHIFT..=Params::MAX_ENTRIES_SHIFT {
      let concrete_array: Slots<Atomic<u64>> = create(1 << shift);
      let abstract_array: Slots<AtomicUsize> = create(1 << shift);

      assert_eq!(concrete_array.len().get(), 1 << shift);
      assert_eq!(abstract_array.len().get(), 1 << shift);
    }
  }
}
