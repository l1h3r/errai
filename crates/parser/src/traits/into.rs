use alloc::borrow::Cow;

use crate::types::FrameId;

// =============================================================================
// IntoOwned
// =============================================================================

/// Conversion from borrowed data to owned data.
pub trait IntoOwned {
  /// The owned equivalent of `Self`.
  type Owned: 'static;

  /// Create owned data from borrowed data.
  fn into_owned(self) -> Self::Owned;
}

// =============================================================================
// Implementations for Crate Types
// =============================================================================

impl<const S: usize> IntoOwned for FrameId<S> {
  type Owned = Self;

  #[inline]
  fn into_owned(self) -> Self::Owned {
    self
  }
}

// =============================================================================
// Implementations for Rust Types
// =============================================================================

impl<T> IntoOwned for Option<T>
where
  T: IntoOwned,
{
  type Owned = Option<T::Owned>;

  #[inline]
  fn into_owned(self) -> Self::Owned {
    self.map(T::into_owned)
  }
}

impl<T> IntoOwned for Vec<T>
where
  T: IntoOwned,
{
  type Owned = Vec<T::Owned>;

  #[inline]
  fn into_owned(self) -> Self::Owned {
    self.into_iter().map(T::into_owned).collect()
  }
}

impl<T> IntoOwned for Cow<'_, T>
where
  T: ToOwned + ?Sized + 'static,
{
  type Owned = Cow<'static, T>;

  #[inline]
  fn into_owned(self) -> <Self as IntoOwned>::Owned {
    Cow::Owned(Cow::into_owned(self))
  }
}

copy_into_owned!(u8, u16, u32, u64);
copy_into_owned!(i8, i16, i32, i64);

copy_into_owned! {
  core::num::NonZeroU8,
  core::num::NonZeroU16,
  core::num::NonZeroU32,
  core::num::NonZeroU64
}

copy_into_owned! {
  core::num::NonZeroI8,
  core::num::NonZeroI16,
  core::num::NonZeroI32,
  core::num::NonZeroI64
}
