use dyn_clone::DynClone;
use dyn_clone::clone_trait_object;
use std::any::Any;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

// -----------------------------------------------------------------------------
// Item
// -----------------------------------------------------------------------------

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
pub trait Item: Any + Debug + DynClone + Send + Sync + 'static {
  /// Tests for `self` and `other` values to be equal.
  ///
  /// This is stricter than [`PartialEq`] because the types must be identical.
  fn dyn_eq(&self, other: &dyn Item) -> bool;
}

clone_trait_object!(Item);

impl<T> Item for T
where
  T: Any + Debug + DynClone + Send + Sync + 'static,
  T: PartialEq,
{
  #[inline]
  fn dyn_eq(&self, other: &dyn Item) -> bool {
    (other as &dyn Any)
      .downcast_ref::<T>()
      .map_or(false, |other| PartialEq::eq(self, other))
  }
}

// -----------------------------------------------------------------------------
// Term
// -----------------------------------------------------------------------------

/// Dynamically typed value that can be sent between processes.
///
/// [`Term`] wraps a boxed [`Item`] and provides type-safe downcasting APIs
/// for inspecting or extracting the contained value. All values stored in
/// a [`Term`] must implement [`Send`], [`Sync`], [`Debug`], and [`Clone`].
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
#[derive(Clone)]
#[repr(transparent)]
pub struct Term {
  data: Box<dyn Item>,
}

impl Term {
  /// Creates a new term wrapping the given value.
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

  /// Returns `true` if the inner type is the same as `T`.
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
    (&*self.data as &dyn Any).is::<T>()
  }

  /// Returns some reference to the inner value if it is of type `T`,
  /// or `None` if it isn’t.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Term;
  ///
  /// let term = Term::new("hello");
  ///
  /// // Successful downcast
  /// assert_eq!(term.downcast_ref::<&str>(), Some(&"hello"));
  ///
  /// // Failed downcast
  /// assert_eq!(term.downcast_ref::<i32>(), None);
  /// ```
  #[inline]
  pub fn downcast_ref<T>(&self) -> Option<&T>
  where
    T: 'static,
  {
    (&*self.data as &dyn Any).downcast_ref()
  }

  /// Returns some mutable reference to the inner value if it is of type `T`,
  /// or `None` if it isn’t.
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
    (&mut *self.data as &mut dyn Any).downcast_mut()
  }

  /// Downcasts the term to a concrete type.
  ///
  /// # Safety
  ///
  /// The contained value **must** be of type `T`.
  /// Calling this method with the incorrect type is *undefined behavior*.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Term;
  ///
  /// let term = Term::new("hello");
  ///
  /// // SAFETY: we know the type
  /// let boxed = unsafe { term.downcast_unchecked::<&str>() };
  /// assert_eq!(*boxed, "hello");
  /// ```
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
        Err(ref error) => Self::new(Self::fallback_error(error)),
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
        None => Self::new(Self::fallback_error(error)),
      },
    }
  }

  #[cold]
  fn fallback_error(error: &dyn Any) -> String {
    format!("unknown error ({error:?})")
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

impl PartialEq for Term {
  #[inline]
  fn eq(&self, other: &Self) -> bool {
    self.data.dyn_eq(&*other.data)
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
  fn test_new_str() {
    assert!(Term::new("test").is::<&str>());
  }

  #[test]
  fn test_new_string() {
    assert!(Term::new(String::from("test")).is::<String>());
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
  fn test_is_with_wrong_type() {
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
    assert!(Term::new((1, "hello", 3.14)).is::<(i32, &str, f64)>());
  }

  #[test]
  fn test_is_with_option() {
    assert!(Term::new(Some(123)).is::<Option<i32>>());
  }

  #[test]
  fn test_is_with_result() {
    assert!(Term::new(Ok::<i32, String>(123)).is::<Result<i32, String>>());
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
  fn test_downcast_ref_string() {
    assert_eq!(
      Term::new(String::from("hello")).downcast_ref::<String>(),
      Some(&String::from("hello"))
    );
  }

  #[test]
  fn test_downcast_ref_vec() {
    assert_eq!(
      Term::new(vec![1, 2, 3]).downcast_ref::<Vec<i32>>(),
      Some(&vec![1, 2, 3])
    );
  }

  #[test]
  fn test_downcast_mut_success() {
    let mut term: Term = Term::new(123_i32);

    *term.downcast_mut::<i32>().unwrap() = 100;

    assert_eq!(term.downcast_ref::<i32>(), Some(&100));
  }

  #[test]
  fn test_downcast_mut_failure() {
    assert!(Term::new(123_i32).downcast_mut::<String>().is_none());
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
  fn test_debug() {
    let src: Term = Term::new(123_i32);
    let fmt: String = format!("{src:?}");

    assert_eq!(fmt, "123");
  }

  #[test]
  fn test_display_equals_debug() {
    let src: Term = Term::new(123_i32);
    let fmt: String = format!("{src}");

    assert_eq!(fmt, format!("{src:?}"));
  }

  #[test]
  fn test_equality() {
    let a: Term = Term::new(123_i32);
    let b: Term = Term::new(123_i32);
    let c: Term = Term::new(99_i32);

    assert_eq!(a, b);
    assert_ne!(a, c);
  }
}
