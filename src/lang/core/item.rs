use dyn_clone::DynClone;
use std::any::Any;
use std::fmt::Debug;
use std::fmt::Display;

pub trait Item: Any + Debug + Display + DynClone + Send + Sync + 'static {
  fn as_any(&self) -> &(dyn Any + Send + Sync);
  fn as_mut_any(&mut self) -> &mut (dyn Any + Send + Sync);
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
