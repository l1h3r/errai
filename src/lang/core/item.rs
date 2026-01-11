use dyn_clone::DynClone;
use std::any::Any;
use std::fmt::Debug;
use std::fmt::Display;

/// A dynamically typed value used by the runtime.
///
/// `Item` represents an arbitrary value that can be passed through the runtime
/// in a type-erased form while still supporting cloning, formatting, and safe
/// downcasting.
///
/// All [`Item`]s:
/// - Are `'static` and thread-safe
/// - Support cloning via [`DynClone`]
/// - Can be formatted using [`Debug`] and [`Display`]
/// - Can be dynamically inspected through [`Any`]
pub trait Item: Any + Debug + Display + DynClone + Send + Sync + 'static {
  /// Returns a shared reference to this value as [`Any`].
  ///
  /// This enables runtime type inspection and safe downcasting.
  fn as_any(&self) -> &(dyn Any + Send + Sync);

  /// Returns a mutable reference to this value as [`Any`].
  ///
  /// This enables mutable runtime downcasting.
  fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync);

  /// Converts this value into a boxed [`Any`].
  ///
  /// This consumes the boxed item and erases its concrete type, allowing it
  /// to be transferred or stored without retaining the [`Item`] interface.
  fn into_any(self: Box<Self>) -> Box<dyn Any + Send + Sync>;
}

impl<T> Item for T
where
  T: Any + Debug + Display + DynClone + Send + Sync + 'static,
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
}
