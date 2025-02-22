use core::fmt::Debug;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;
use core::ops::Index;
use core::slice::Iter;
use core::slice::SliceIndex;
use std::io::Cursor;

use crate::types::Bytes;

// =============================================================================
// Slice
// =============================================================================

/// A borrowed slice of bytes.
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Slice {
  inner: [u8],
}

impl Slice {
  pub(crate) const fn new(inner: &[u8]) -> &Self {
    // SAFETY: Self is a DST with same representation as inner.
    unsafe { &*(inner as *const [u8] as *const Self) }
  }

  /// Create a new empty `Slice`.
  #[inline]
  pub const fn empty() -> &'static Self {
    Self::new(&[])
  }

  /// Returns the number of bytes in the slice.
  #[inline]
  pub const fn len(&self) -> usize {
    self.inner.len()
  }

  /// Returns `true` if the slice has a length of 0.
  #[inline]
  pub const fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  /// Returns an iterator over the bytes in the slice.
  #[inline]
  pub fn iter(&self) -> Iter<'_, u8> {
    self.inner.iter()
  }

  /// Returns a sublice without doing bounds checking.
  pub unsafe fn get_unchecked<I>(&self, index: I) -> &Self
  where
    I: SliceIndex<[u8], Output = [u8]>,
  {
    Self::new(self.inner.get_unchecked(index))
  }

  /// Wrap the slice in a `Cursor` that implements [`Read`][std::io::Read].
  #[inline]
  pub fn cursor(&self) -> Cursor<&Self> {
    Cursor::new(self)
  }

  /// Returns a subslice advanced by `count` bytes.
  #[inline]
  pub fn skip(&self, count: usize) -> &Self {
    // SAFETY: index is constrained within the bounds of the slice.
    unsafe { self.get_unchecked(count.min(self.len() - 1)..) }
  }

  /// Returns a subslice of up to `count` bytes.
  #[inline]
  pub fn take(&self, count: usize) -> &Self {
    // SAFETY: index is constrained within the bounds of the slice.
    unsafe { self.get_unchecked(..count.min(self.len())) }
  }

  /// Returns a subslice of `count` bytes offset by `start`.
  #[inline]
  pub fn view(&self, start: usize, count: usize) -> &Self {
    self.skip(start).take(count)
  }
}

impl Debug for Slice {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("Slice { .. }")
  }
}

impl ToOwned for Slice {
  type Owned = Bytes;

  #[inline]
  fn to_owned(&self) -> Self::Owned {
    Bytes::new(self.inner.into())
  }
}

impl<I> Index<I> for Slice
where
  I: SliceIndex<[u8], Output = [u8]>,
{
  type Output = Self;

  #[inline]
  fn index(&self, index: I) -> &Self::Output {
    Self::new(self.inner.index(index))
  }
}

impl AsRef<[u8]> for Slice {
  #[inline]
  fn as_ref(&self) -> &[u8] {
    &self.inner
  }
}
