//! Type-erased runtime value container used for inter-process communication.
//!
//! This module provides [`Term`], a dynamically typed value container that
//! can safely traverse process boundaries. Terms support cloning, debugging,
//! and type-safe downcasting.
//!
//! # Use Cases
//!
//! [`Term`] is designed for scenarios where the concrete type isn't known
//! at compile time:
//!
//! - Generic message passing between processes
//! - Dynamic process dictionaries
//! - Polymorphic exit reasons and error values
//! - Inter-process data structures with heterogeneous elements
//!
//! # Type Safety
//!
//! [`Term`] uses Rust's [`Any`] trait for runtime type checking. Values
//! can be safely extracted using [`downcast_ref()`] and [`downcast_mut()`],
//! which return [`None`] if the type doesn't match.
//!
//! # Examples
//!
//! ```
//! use errai::core::Term;
//!
//! // Create terms from various types
//! let num = Term::new(42_i32);
//! let text = Term::new(String::from("hello"));
//!
//! // Type-safe downcasting
//! assert_eq!(num.downcast_ref::<i32>(), Some(&42));
//! assert_eq!(num.downcast_ref::<String>(), None);
//!
//! // Terms are cloneable
//! let cloned = num.clone();
//! assert_eq!(cloned.downcast_ref::<i32>(), Some(&42));
//! ```
//!
//! [`downcast_ref()`]: Term::downcast_ref
//! [`downcast_mut()`]: Term::downcast_mut

use dyn_clone::clone_box;
use std::any::Any;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::core::Item;

/// Dynamically typed value that can be sent between processes.
///
/// [`Term`] wraps a boxed [`Item`] and provides type-safe downcasting APIs
/// for inspecting or extracting the contained value. All values stored in
/// a [`Term`] must implement [`Send`], [`Sync`], [`Debug`], and [`Clone`].
///
/// # Cloning Behavior
///
/// Cloning a [`Term`] performs a deep clone of the contained value using
/// the [`DynClone`] trait. This ensures each process has its own copy of
/// the data after message passing.
///
/// # Type Erasure
///
/// The concrete type is erased at the [`Term`] boundary but can be recovered
/// at runtime using the downcasting methods:
///
/// - [`is()`]: Check if the value is of type `T`
/// - [`downcast_ref()`]: Borrow the value as `&T`
/// - [`downcast_mut()`]: Borrow the value as `&mut T`
/// - [`downcast_unchecked()`]: Extract the value as `Box<T>` (unsafe)
///
/// # Examples
///
/// ```
/// use errai::core::Term;
///
/// let mut term = Term::new(vec![1, 2, 3]);
///
/// // Check the type
/// assert!(term.is::<Vec<i32>>());
///
/// // Modify through mutable reference
/// if let Some(vec) = term.downcast_mut::<Vec<i32>>() {
///   vec.push(4);
/// }
///
/// assert_eq!(term.downcast_ref::<Vec<i32>>(), Some(&vec![1, 2, 3, 4]));
/// ```
///
/// [`DynClone`]: dyn_clone::DynClone
/// [`is()`]: Self::is
/// [`downcast_ref()`]: Self::downcast_ref
/// [`downcast_mut()`]: Self::downcast_mut
/// [`downcast_unchecked()`]: Self::downcast_unchecked
#[repr(transparent)]
pub struct Term {
  data: Box<dyn Item>,
}

impl Term {
  /// Creates a new term wrapping the given value.
  ///
  /// The value must implement [`Item`], which is automatically satisfied
  /// by most types that are [`Debug`] + [`Clone`] + [`Send`] + [`Sync`].
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Term;
  ///
  /// let num = Term::new(42);
  /// let text = Term::new("hello");
  /// let data = Term::new(vec![1, 2, 3]);
  /// ```
  #[inline]
  pub fn new<T>(data: T) -> Self
  where
    T: Item,
  {
    Self {
      data: Box::new(data),
    }
  }

  /// Returns `true` if the contained value is of type `T`.
  ///
  /// This is equivalent to checking `downcast_ref::<T>().is_some()` but
  /// more efficient as it only performs the type check.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Term;
  ///
  /// let term = Term::new(42_i32);
  ///
  /// assert!(term.is::<i32>());
  /// assert!(!term.is::<String>());
  /// ```
  #[inline]
  pub fn is<T>(&self) -> bool
  where
    T: 'static,
  {
    self.data.as_any().is::<T>()
  }

  /// Returns a shared reference to the contained value of type `T`.
  ///
  /// Returns [`None`] if the value has a different concrete type.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Term;
  ///
  /// let term = Term::new(String::from("hello"));
  ///
  /// // Successful downcast
  /// assert_eq!(term.downcast_ref::<String>(), Some(&String::from("hello")));
  ///
  /// // Failed downcast
  /// assert_eq!(term.downcast_ref::<i32>(), None);
  /// ```
  #[inline]
  pub fn downcast_ref<T>(&self) -> Option<&T>
  where
    T: 'static,
  {
    self.data.as_any().downcast_ref()
  }

  /// Returns a mutable reference to the contained value of type `T`.
  ///
  /// Returns [`None`] if the value has a different concrete type.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Term;
  ///
  /// let mut term = Term::new(vec![1, 2, 3]);
  ///
  /// if let Some(vec) = term.downcast_mut::<Vec<i32>>() {
  ///   vec.push(4);
  /// }
  ///
  /// assert_eq!(term.downcast_ref::<Vec<i32>>(), Some(&vec![1, 2, 3, 4]));
  /// ```
  #[inline]
  pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
  where
    T: 'static,
  {
    self.data.as_mut_any().downcast_mut()
  }

  /// Converts this term into a boxed value of type `T` without checks.
  ///
  /// # Safety
  ///
  /// The contained value **must** be of type `T`. Supplying an incorrect
  /// type results in undefined behavior.
  ///
  /// Use [`downcast_ref()`] or [`is()`] to verify the type first, or use
  /// this method only when the type is guaranteed by construction.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Term;
  ///
  /// let term = Term::new(String::from("hello"));
  ///
  /// // Safe: we know the type
  /// let boxed = unsafe { term.downcast_unchecked::<String>() };
  /// assert_eq!(*boxed, "hello");
  /// ```
  ///
  /// [`downcast_ref()`]: Self::downcast_ref
  /// [`is()`]: Self::is
  #[inline]
  pub unsafe fn downcast_unchecked<T>(self) -> Box<T>
  where
    T: 'static,
  {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { Box::from_raw(Box::into_raw(self.data).cast::<T>()) }
  }

  /// Creates a term from a boxed dynamically typed error value.
  #[inline]
  pub(crate) fn new_error(error: Box<dyn Any + Send>) -> Self {
    match error.downcast::<&str>() {
      Ok(error) => Self::new(error),
      Err(error) => match error.downcast::<String>() {
        Ok(error) => Self::new(error),
        Err(error) => Self::new(format!("unknown error ({error:?})")),
      },
    }
  }

  /// Creates a term from a borrowed dynamically typed error reference.
  #[inline]
  pub(crate) fn new_error_ref(error: &(dyn Any + Send)) -> Self {
    match error.downcast_ref::<&str>() {
      Some(error) => Self::new(error.to_owned()),
      None => match error.downcast_ref::<String>() {
        Some(error) => Self::new(error.to_owned()),
        None => Self::new(format!("unknown error ({error:?})")),
      },
    }
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use crate::core::Atom;
  use crate::core::Term;

  #[test]
  fn test_new_i32() {
    assert!(Term::new(123_i32).is::<i32>());
  }

  #[test]
  fn test_new_string() {
    assert!(Term::new(String::from("hello")).is::<String>());
  }

  #[test]
  fn test_new_vec() {
    assert!(Term::new(vec![1, 2, 3]).is::<Vec<i32>>());
  }

  #[test]
  fn test_new_atom() {
    assert!(Term::new(Atom::new("test")).is::<Atom>());
  }

  #[test]
  fn test_is_wrong_type() {
    let term: Term = Term::new(123_i32);

    assert!(!term.is::<String>());
    assert!(!term.is::<f64>());
  }

  #[test]
  fn test_is_with_similar_types() {
    let term: Term = Term::new(123_i32);

    assert!(term.is::<i32>());
    assert!(!term.is::<i64>());
    assert!(!term.is::<u32>());
  }

  #[test]
  fn test_is_with_tuple() {
    let term: Term = Term::new((1, "hello", 3.14));
    assert!(term.is::<(i32, &str, f64)>());
  }

  #[test]
  fn test_is_with_option() {
    let term: Term = Term::new(Some(123));

    assert!(term.is::<Option<i32>>());
    assert_eq!(term.downcast_ref::<Option<i32>>(), Some(&Some(123)));
  }

  #[test]
  fn test_is_with_result() {
    let term: Term = Term::new(Ok::<i32, String>(123));
    assert!(term.is::<Result<i32, String>>());
  }

  #[test]
  fn test_downcast_ref_success() {
    assert_eq!(Term::new(123_i32).downcast_ref::<i32>(), Some(&123));
  }

  #[test]
  fn test_downcast_ref_failure() {
    assert_eq!(Term::new(123_i32).downcast_ref::<String>(), None);
  }

  #[test]
  fn test_downcast_mut_success() {
    let mut term: Term = Term::new(123_i32);

    *term.downcast_mut::<i32>().unwrap() = 100;

    assert_eq!(term.downcast_ref::<i32>(), Some(&100));
  }

  #[test]
  fn test_downcast_mut_failure() {
    let mut term: Term = Term::new(123_i32);
    assert!(term.downcast_mut::<String>().is_none());
  }

  #[test]
  fn test_downcast_ref_string() {
    let term: Term = Term::new(String::from("hello"));
    assert_eq!(term.downcast_ref::<String>(), Some(&String::from("hello")));
  }

  #[test]
  fn test_downcast_ref_vec() {
    let term: Term = Term::new(vec![1, 2, 3]);
    assert_eq!(term.downcast_ref::<Vec<i32>>(), Some(&vec![1, 2, 3]));
  }

  #[test]
  fn test_downcast_mut_vec() {
    let mut term: Term = Term::new(vec![1, 2, 3]);

    term.downcast_mut::<Vec<i32>>().unwrap().push(4);

    assert_eq!(term.downcast_ref::<Vec<i32>>(), Some(&vec![1, 2, 3, 4]));
  }

  #[test]
  fn test_clone() {
    let src: Term = Term::new(123_i32);
    let dst: Term = src.clone();

    assert_eq!(src.downcast_ref::<i32>(), Some(&123));
    assert_eq!(dst.downcast_ref::<i32>(), Some(&123));
  }

  #[test]
  fn test_clone_ownership() {
    let src: Term = Term::new(vec![1, 2, 3]);

    let dst: Term = {
      let mut term: Term = src.clone();
      term.downcast_mut::<Vec<i32>>().unwrap().push(4);
      term
    };

    assert_eq!(src.downcast_ref::<Vec<i32>>(), Some(&vec![1, 2, 3]));
    assert_eq!(dst.downcast_ref::<Vec<i32>>(), Some(&vec![1, 2, 3, 4]));
  }

  #[test]
  fn test_display() {
    let src: Term = Term::new(123_i32);
    let fmt: String = format!("{src}");

    assert_eq!(fmt, "123");
  }

  #[test]
  fn test_debug() {
    let src: Term = Term::new(123_i32);
    let fmt: String = format!("{src:?}");

    assert_eq!(fmt, "123");
  }

  #[ignore]
  #[test]
  fn test_equality() {
    let a: Term = Term::new(123_i32);
    let b: Term = Term::new(123_i32);
    let c: Term = Term::new(99_i32);

    // assert_eq!(a, b); // TODO: uncomment when Terms are comparable
    // assert_ne!(a, c); // TODO: uncomment when Terms are comparable
  }
}
