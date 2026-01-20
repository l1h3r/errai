//! Trait defining type-erased runtime values usable within [`Term`].
//!
//! This module provides the internal [`Item`] trait that enables dynamic
//! typing in [`Term`]. Most users will work with [`Term`] directly rather
//! than implementing [`Item`] manually.
//!
//! [`Term`]: crate::core::Term

use dyn_clone::DynClone;
use std::any::Any;
use std::fmt::Debug;

/// Trait implemented by all values stored inside a [`Term`].
///
/// This trait enables safe dynamic typing, cloning, and thread-safe sharing
/// of values across process boundaries.
///
/// # Automatic Implementation
///
/// [`Item`] is automatically implemented for all types that satisfy:
///
/// - [`Any`]: Required for downcasting
/// - [`Debug`]: Required for diagnostic output
/// - [`DynClone`]: Required for cloning trait objects
/// - [`Send`] + [`Sync`]: Required for inter-process communication
/// - `'static`: Required for type erasure
///
/// Most types can be used in [`Term`] without explicit [`Item`] implementation.
///
/// # Examples
///
/// ```
/// use errai::core::Term;
///
/// // These types automatically implement Item:
/// let t1 = Term::new(42_i32);
/// let t2 = Term::new(String::from("hello"));
/// let t3 = Term::new(vec![1, 2, 3]);
/// ```
///
/// [`Term`]: crate::core::Term
pub trait Item: Any + Debug + DynClone + Send + Sync + 'static {
  /// Returns a shared reference to this value as [`Any`].
  ///
  /// This enables downcasting to the concrete type.
  fn as_any(&self) -> &(dyn Any + Send + Sync);

  /// Returns a mutable reference to this value as [`Any`].
  ///
  /// This enables mutable downcasting to the concrete type.
  fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync);

  /// Converts this value into a boxed [`Any`] trait object.
  ///
  /// This enables owned downcasting to the concrete type.
  fn into_any(self: Box<Self>) -> Box<dyn Any + Send + Sync>;

  /// Tests for `self` and `other` values to be equal.
  ///
  /// This is stricter than [`PartialEq`] because the types must be identical.
  fn dyn_eq(&self, other: &dyn Any) -> bool;
}

impl PartialEq for dyn Item {
  #[inline]
  fn eq(&self, other: &Self) -> bool {
    self.dyn_eq(other)
  }
}

impl<T> Item for T
where
  T: Any + Debug + DynClone + Send + Sync + 'static,
  T: PartialEq,
{
  #[inline]
  fn as_any(&self) -> &(dyn Any + Send + Sync) {
    self
  }

  #[inline]
  fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync) {
    self
  }

  #[inline]
  fn into_any(self: Box<Self>) -> Box<dyn Any + Send + Sync> {
    self
  }

  #[inline]
  fn dyn_eq(&self, other: &dyn Any) -> bool {
    other
      .downcast_ref::<T>()
      .map_or(false, |other| PartialEq::eq(self, other))
  }
}
