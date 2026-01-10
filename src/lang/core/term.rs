use dyn_clone::clone_box;
use std::any::Any;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::lang::Item;

/// A boxed piece of arbitrary data.
#[repr(transparent)]
pub struct Term {
  data: Box<dyn Item>,
}

impl Term {
  /// Creates a new `Term` from the given `data`.
  #[inline]
  pub fn new<T>(data: T) -> Self
  where
    T: Item,
  {
    Self {
      data: Box::new(data),
    }
  }

  /// Creates a new `Term` from a dynamic error value.
  #[inline]
  pub fn new_error(error: Box<dyn Any + Send>) -> Self {
    match error.downcast::<&str>() {
      Ok(error) => Self::new(error),
      Err(error) => match error.downcast::<String>() {
        Ok(error) => Self::new(error),
        Err(error) => Self::new(format!("{error:?}")),
      },
    }
  }

  /// Returns `true` if the inner type matches `T`.
  #[inline]
  pub fn is<T>(&self) -> bool
  where
    T: 'static,
  {
    self.data.as_any().is::<T>()
  }

  /// Returns some reference to the inner value if it is of type `T`, or `None` if it isn't.
  #[inline]
  pub fn downcast_ref<T>(&self) -> Option<&T>
  where
    T: 'static,
  {
    self.data.as_any().downcast_ref()
  }

  /// Returns some mutable reference to the inner value if it is of type `T`, or `None` if it isn't.
  #[inline]
  pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
  where
    T: 'static,
  {
    self.data.as_mut_any().downcast_mut()
  }

  /// Downcasts the boxed term to a concrete type.
  ///
  /// # Safety
  ///
  /// The contained value must be of type `T`. Calling this method with the
  /// incorrect type is undefined behavior.
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
    write!(f, "Term({:?})", self.data)
  }
}

impl Display for Term {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    write!(f, "Term({})", self.data)
  }
}
