use std::fmt::Debug;
use std::fmt::Display;

use crate::lang::ExternalPid;
use crate::lang::InternalPid;

mod private {
  pub trait Sealed {}
}

impl private::Sealed for ExternalPid {}
impl private::Sealed for InternalPid {}

pub trait ProcessId: private::Sealed + Debug + Display {
  const DISTRIBUTED: bool;

  /// Converts `self` into an internal PID.
  fn into_internal(self) -> InternalPid;

  /// Converts `self` into an external PID.
  fn into_external(self) -> Option<ExternalPid>;
}
