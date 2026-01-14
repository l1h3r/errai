use dyn_clone::clone_box;
use std::any::Any;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::lang::Item;

/// A boxed, type-erased value implementing [`Item`].
///
/// `Term` is a transparent wrapper around a boxed [`Item`] trait object,
/// allowing heterogeneous values to be stored, cloned, and dynamically
/// inspected at runtime.
///
/// The inner value can be queried or downcast using the provided helper
/// methods.
///
/// [`Item`]: crate::lang::Item
#[repr(transparent)]
pub struct Term {
  data: Box<dyn Item>,
}

impl Term {
  /// Creates a new [`Term`] from the given `data`.
  ///
  /// The value is boxed and stored as a trait object implementing [`Item`].
  #[inline]
  pub fn new<T>(data: T) -> Self
  where
    T: Item,
  {
    Self {
      data: Box::new(data),
    }
  }

  /// Creates a new [`Term`] from a dynamically typed error value.
  ///
  /// If the error is a `&str` or [`String`], its message is preserved.
  /// Otherwise, the error is formatted using [`Debug`] and wrapped in a
  /// generic `"unknown error (...)"` message.
  ///
  /// This is primarily intended for converting opaque error values into a
  /// displayable [`Term`].
  #[inline]
  pub fn new_error(error: Box<dyn Any + Send>) -> Self {
    match error.downcast::<&str>() {
      Ok(error) => Self::new(error),
      Err(error) => match error.downcast::<String>() {
        Ok(error) => Self::new(error),
        Err(error) => Self::new(format!("unknown error ({error:?})")),
      },
    }
  }

  /// Creates a new [`Term`] from a dynamically typed error reference.
  ///
  /// This behaves like [`Term::new_error`], but operates on a borrowed error
  /// value instead of taking ownership.
  #[inline]
  pub fn new_error_ref(error: &(dyn Any + Send)) -> Self {
    match error.downcast_ref::<&str>() {
      Some(error) => Self::new(error.to_owned()),
      None => match error.downcast_ref::<String>() {
        Some(error) => Self::new(error.to_owned()),
        None => Self::new(format!("unknown error ({error:?})")),
      },
    }
  }

  /// Returns `true` if the inner value is of type `T`.
  ///
  /// This performs a runtime type check using [`Any`].
  #[inline]
  pub fn is<T>(&self) -> bool
  where
    T: 'static,
  {
    self.data.as_any().is::<T>()
  }

  /// Returns a shared reference to the inner value if it is of type `T`.
  ///
  /// If the stored value does not match `T`, this returns `None`.
  #[inline]
  pub fn downcast_ref<T>(&self) -> Option<&T>
  where
    T: 'static,
  {
    self.data.as_any().downcast_ref()
  }

  /// Returns a mutable reference to the inner value if it is of type `T`.
  ///
  /// If the stored value does not match `T`, this returns `None`.
  #[inline]
  pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
  where
    T: 'static,
  {
    self.data.as_mut_any().downcast_mut()
  }

  /// Downcasts the boxed [`Term`] into a concrete boxed value of type `T`.
  ///
  /// # Safety
  ///
  /// The contained value **must** be of type `T`.
  ///
  /// Calling this method with an incorrect type results in undefined behavior,
  /// as the cast bypasses all runtime checks.
  #[inline]
  pub unsafe fn downcast_unchecked<T>(self) -> Box<T>
  where
    T: 'static,
  {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { Box::from_raw(Box::into_raw(self.data).cast::<T>()) }
  }
}

impl Clone for Term {
  #[inline]
  fn clone(&self) -> Self {
    Self {
      data: clone_box(&*self.data),
    }
  }
}

impl Debug for Term {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Debug::fmt(&*self.data, f)
  }
}

impl Display for Term {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Debug::fmt(&*self.data, f)
  }
}
