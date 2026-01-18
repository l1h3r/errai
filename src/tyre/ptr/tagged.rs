use core::cmp::Ordering;
use core::fmt::Debug;
use core::fmt::Formatter;
use core::fmt::Pointer;
use core::fmt::Result;
use core::hash::Hash;
use core::hash::Hasher;
use core::mem::MaybeUninit;
use core::panic::RefUnwindSafe;
use core::panic::UnwindSafe;
use core::ptr;
use core::sync::atomic::AtomicPtr;

#[cfg(target_pointer_width = "32")]
const ASSUMED_FREE_BITS: u32 = 2;

#[cfg(target_pointer_width = "64")]
const ASSUMED_FREE_BITS: u32 = 3;

/// Calculates the number of trailing zero bits in the alignment of `T`.
///
/// This is used to ensure that we have enough bits to store the tag value.
#[inline]
const fn pointer_free_bits<T>() -> u32 {
  align_of::<T>().trailing_zeros()
}

/// Returns `true` if alignment of `T` is sufficient to store a tag value.
#[inline]
const fn pointer_alignment_valid<T>() -> bool {
  pointer_free_bits::<T>() >= ASSUMED_FREE_BITS
}

/// Computes the new value of `addr` tagged with the value `tag`.
#[inline]
const fn mask<T>(addr: usize, tag: u8) -> usize
where
  T: ?Sized,
{
  (addr & TaggedPtr::<T>::PTR_MASK) | (tag as usize & TaggedPtr::<T>::TAG_MASK)
}

/// Sets the tag value of `ptr` to `tag`.
#[inline]
fn set<T>(ptr: *mut T, tag: u8) -> *mut T
where
  T: ?Sized,
{
  ptr.map_addr(|addr| mask::<T>(addr, tag))
}

/// Removes the tag from `ptr`, returning the original pointer.
#[inline]
fn del<T>(ptr: *mut T) -> *mut T
where
  T: ?Sized,
{
  ptr.map_addr(|addr| addr & TaggedPtr::<T>::PTR_MASK)
}

/// A raw pointer with a tag value.
///
/// This type has the same size and bit validity as a [`*mut T`][`*mut`].
#[repr(transparent)]
pub struct TaggedPtr<T>
where
  T: ?Sized,
{
  inner: *mut T,
}

impl<T> TaggedPtr<T>
where
  T: ?Sized,
{
  /// Creates a new `TaggedPtr` from `ptr`, assuming it is properly aligned.
  ///
  /// # Safety
  ///
  /// The pointer must have sufficient alignment to store a tag value.
  #[must_use]
  #[inline]
  const unsafe fn from_ptr_unchecked(ptr: *mut T) -> Self {
    Self { inner: ptr }
  }

  // ---------------------------------------------------------------------------
  // Pointer Tag API
  // ---------------------------------------------------------------------------

  /// The total number of bits available to store a tag value.
  pub const BITS: u32 = ASSUMED_FREE_BITS;

  /// A bitmask covering the tag value bits of a pointer.
  pub const TAG_MASK: usize = 1_usize.strict_shl(Self::BITS).strict_sub(1);

  /// A bitmask covering the address bits of a pointer.
  pub const PTR_MASK: usize = !Self::TAG_MASK;

  /// Returns `true` if `TaggedPtr<T>` has valid alignment.
  ///
  /// This guarantees that the pointer type `T` has sufficient alignment to
  /// store a tag value. If this function returns `true`, [`new`][Self::new] is
  /// guaranteed to return `Some(_)`.
  #[inline]
  pub const fn is_valid() -> bool
  where
    T: Sized,
  {
    const { pointer_alignment_valid::<T>() }
  }

  /// Returns the tag value of the tagged pointer.
  #[inline]
  pub fn tag(self) -> u8 {
    (self.inner.addr() & Self::TAG_MASK) as u8
  }

  /// Creates a new `TaggedPtr` with the tag value set to `tag`.
  #[must_use]
  #[inline]
  pub fn with(ptr: *mut T, tag: u8) -> Option<Self>
  where
    T: Sized,
  {
    if let Some(this) = Self::new(ptr) {
      Some(this.set(tag))
    } else {
      None
    }
  }

  /// Creates a new `TaggedPtr` with the tag value set to `tag`.
  ///
  /// # Safety
  ///
  /// `ptr` must have sufficient alignment to hold a tag value.
  #[must_use]
  #[inline]
  pub unsafe fn with_unchecked(ptr: *mut T, tag: u8) -> Self
  where
    T: Sized,
  {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { Self::new_unchecked(ptr) }.set(tag)
  }

  /// Sets the tagged pointer value to `tag`.
  ///
  /// Any excess bits in `tag` are truncated to fit the available tag space.
  #[must_use]
  #[inline]
  pub fn set(self, tag: u8) -> Self {
    // SAFETY: `self` is already known to have valid alignment.
    unsafe { Self::from_ptr_unchecked(set(self.as_ptr_tagged(), tag)) }
  }

  /// Removes the tag value from the `self`, returning the underlying pointer.
  #[must_use]
  #[inline]
  pub fn del(self) -> Self {
    // SAFETY: `self` is already known to have valid alignment.
    unsafe { Self::from_ptr_unchecked(del(self.as_ptr_tagged())) }
  }

  // ---------------------------------------------------------------------------
  // Core Pointer API
  // ---------------------------------------------------------------------------

  /// Creates a null mutable tagged pointer.
  ///
  /// # Panics
  ///
  /// This method will panic if `*mut T` does not have sufficient alignment to
  /// hold a tag value.
  #[must_use]
  #[inline]
  pub const fn null() -> Self
  where
    T: Sized,
  {
    if let Some(this) = Self::new(ptr::null_mut()) {
      this
    } else {
      panic!("TaggedPtr::null requires that the pointer can hold a tag value");
    }
  }

  /// Creates a new `TaggedPtr` if `ptr` has sufficient alignment to hold a tag
  /// value.
  #[must_use]
  #[inline]
  pub const fn new(ptr: *mut T) -> Option<Self>
  where
    T: Sized,
  {
    if Self::is_valid() {
      // SAFETY: The pointer is already checked and has valid alignment.
      Some(unsafe { Self::new_unchecked(ptr) })
    } else {
      None
    }
  }

  /// Creates a new `TaggedPtr`.
  ///
  /// # Safety
  ///
  /// `ptr` must have sufficient alignment to hold a tag value.
  #[must_use]
  #[inline]
  pub const unsafe fn new_unchecked(ptr: *mut T) -> Self
  where
    T: Sized,
  {
    debug_assert!(
      Self::is_valid(),
      "TaggedPtr::new_unchecked requires that the pointer is properly aligned"
    );

    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { Self::from_ptr_unchecked(ptr) }
  }

  /// Acquires the underlying `*mut` pointer, removing the tag.
  #[must_use]
  #[inline]
  pub fn as_ptr(self) -> *mut T {
    self.del().inner
  }

  /// Acquires the underlying `*mut` pointer **without** removing the tag.
  ///
  /// # Safety
  ///
  /// The returned pointer is not [convertible to a reference], and care must be
  /// taken when dereferencing it.
  ///
  /// [convertible to a reference]: std::ptr#pointer-to-reference-conversion
  #[must_use]
  #[inline]
  pub const fn as_ptr_tagged(self) -> *mut T {
    self.inner
  }

  /// Returns `None` if the pointer is null, or else returns a shared reference
  /// to the value wrapped in `Some`.
  ///
  /// # Safety
  ///
  /// When calling this method, you have to ensure that *either* the pointer
  /// is null *or* the pointer is [convertible to a reference].
  ///
  /// [convertible to a reference]: std::ptr#pointer-to-reference-conversion
  #[inline]
  pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.as_ptr().as_ref() }
  }

  /// Returns `None` if the pointer is null, or else returns a unique reference
  /// to the value wrapped in `Some`.
  ///
  /// # Safety
  ///
  /// When calling this method, you have to ensure that *either* the pointer
  /// is null *or* the pointer is [convertible to a reference].
  ///
  /// [convertible to a reference]: std::ptr#pointer-to-reference-conversion
  #[inline]
  pub unsafe fn as_mut<'a>(self) -> Option<&'a mut T> {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.as_ptr().as_mut() }
  }

  /// Casts to a pointer of another type.
  ///
  /// # Safety
  ///
  /// The alignment of `U` must be greater or equal than the alignment of `T`.
  #[inline]
  pub const unsafe fn cast<U>(self) -> TaggedPtr<U> {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { TaggedPtr::new_unchecked(self.inner.cast()) }
  }

  /// Casts from a pointer-to-`T` to a pointer-to-`[T; N]`.
  ///
  /// # Safety
  ///
  /// See [`cast`][Self::cast].
  #[must_use]
  #[inline]
  pub const unsafe fn cast_array<const N: usize>(self) -> TaggedPtr<[T; N]>
  where
    T: Sized,
  {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.cast() }
  }

  /// Casts from a type to its maybe-uninitialized version.
  ///
  /// # Safety
  ///
  /// See [`cast`][Self::cast].
  #[must_use]
  #[inline]
  pub const unsafe fn cast_uninit(self) -> TaggedPtr<MaybeUninit<T>>
  where
    T: Sized,
  {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.cast() }
  }

  /// Returns true if the pointer is null.
  #[inline]
  pub fn is_null(self) -> bool {
    self.as_ptr().is_null()
  }

  /// Returns whether the pointer is properly aligned for `T`.
  #[inline]
  pub fn is_aligned(self) -> bool
  where
    T: Sized,
  {
    self.as_ptr().is_aligned()
  }

  // ---------------------------------------------------------------------------
  // Strict Provenance API
  // ---------------------------------------------------------------------------

  /// Gets the "address" portion of the pointer.
  #[must_use]
  #[inline]
  pub fn addr(self) -> usize {
    self.as_ptr().addr()
  }

  /// Creates a new pointer with the given address and the [provenance] of `self`.
  ///
  /// [provenance]: std::ptr#provenance
  #[must_use]
  #[inline]
  pub fn with_addr(self, addr: usize) -> Self {
    // SAFETY: `self` is already known to have valid alignment.
    unsafe { Self::from_ptr_unchecked(set(self.inner.with_addr(addr), self.tag())) }
  }
}

impl<T> Clone for TaggedPtr<T>
where
  T: ?Sized,
{
  #[inline]
  fn clone(&self) -> Self {
    *self
  }
}

impl<T> Copy for TaggedPtr<T> where T: ?Sized {}

impl<T> Debug for TaggedPtr<T>
where
  T: ?Sized,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Debug::fmt(&self.as_ptr(), f)
  }
}

impl<T> Pointer for TaggedPtr<T>
where
  T: ?Sized,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Pointer::fmt(&self.as_ptr(), f)
  }
}

impl<T> Default for TaggedPtr<T> {
  #[inline]
  fn default() -> Self {
    Self::null()
  }
}

impl<T> Hash for TaggedPtr<T>
where
  T: ?Sized,
{
  #[inline]
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.as_ptr().hash(state);
  }
}

impl<T> PartialEq for TaggedPtr<T>
where
  T: ?Sized,
{
  #[inline]
  fn eq(&self, other: &Self) -> bool {
    self.as_ptr().cast::<()>().eq(&other.as_ptr().cast::<()>())
  }
}

impl<T> Eq for TaggedPtr<T> where T: ?Sized {}

impl<T> PartialOrd for TaggedPtr<T>
where
  T: ?Sized,
{
  #[inline]
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl<T> Ord for TaggedPtr<T>
where
  T: ?Sized,
{
  #[inline]
  fn cmp(&self, other: &Self) -> Ordering {
    self.as_ptr().cast::<()>().cmp(&other.as_ptr().cast::<()>())
  }
}

impl<T> Unpin for TaggedPtr<T> where T: ?Sized {}

impl<T> UnwindSafe for TaggedPtr<T> where T: RefUnwindSafe + ?Sized {}

impl<T> From<TaggedPtr<T>> for AtomicPtr<T> {
  /// Converts a `TaggedPtr<T>` into an `AtomicPtr<T>`.
  #[inline]
  fn from(other: TaggedPtr<T>) -> Self {
    AtomicPtr::new(other.as_ptr_tagged())
  }
}
