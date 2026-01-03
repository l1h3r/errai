use dyn_clone::clone_box;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::lang::Item;

/// A boxed piece of arbitrary data.
#[repr(transparent)]
pub struct Term(Box<dyn Item>);

impl Term {
  /// Creates a new `Term`.
  #[inline]
  pub fn new<T>(value: T) -> Self
  where
    T: Item,
  {
    Self(Box::new(value))
  }
}

impl Clone for Term {
  #[inline]
  fn clone(&self) -> Self {
    Self(clone_box(&*self.0))
  }
}

impl Debug for Term {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    write!(f, "Term({:?})", self.0)
  }
}

impl Display for Term {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    write!(f, "Term({})", self.0)
  }
}
