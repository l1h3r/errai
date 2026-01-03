use dyn_clone::DynClone;
use std::any::Any;
use std::fmt::Debug;
use std::fmt::Display;

pub trait Item: Any + Debug + Display + DynClone + Send + 'static {}

impl<T> Item for T where T: Any + Debug + Display + DynClone + Send + 'static {}
