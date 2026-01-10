use dyn_clone::DynClone;
use std::any::Any;
use std::fmt::Debug;
use std::fmt::Display;

/// `Item`s are arbitrary values that are passed around the runtime.
pub trait Item: Any + Debug + Display + DynClone + Send + Sync + 'static {
  /// Returns a reference to the item as `dyn Any`.
  fn as_any(&self) -> &(dyn Any + Send + Sync);

  /// Returns a mutable reference to the item as `dyn Any`.
  fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync);

  /// Consumes the item and returns it as a boxed `dyn Any`.
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
