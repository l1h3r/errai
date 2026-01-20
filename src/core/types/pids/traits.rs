use std::fmt::Debug;
use std::fmt::Display;

use crate::core::ExternalPid;
use crate::core::InternalPid;

mod private {
  pub trait Sealed {}
}

impl private::Sealed for ExternalPid {}
impl private::Sealed for InternalPid {}

/// Trait implemented by all process identifier representations.
///
/// This sealed trait provides a common interface for [`InternalPid`] and
/// [`ExternalPid`], enabling generic code that works with both local and
/// distributed processes.
///
/// # Sealed Trait
///
/// This trait is sealed and cannot be implemented outside this crate. Only
/// [`InternalPid`] and [`ExternalPid`] implement it.
///
/// # Examples
///
/// ```
/// use errai::core::{InternalPid, ExternalPid, ProcessId};
///
/// fn is_local<P: ProcessId>(pid: P) -> bool {
///   !P::DISTRIBUTED
/// }
///
/// assert!(is_local(InternalPid::EXAMPLE_PROC));
/// ```
pub trait ProcessId: private::Sealed + Copy + Debug + Display {
  /// Indicates whether this process identifier is distributed.
  ///
  /// Returns `true` for [`ExternalPid`] and `false` for [`InternalPid`].
  const DISTRIBUTED: bool;

  /// Converts this identifier into its internal PID representation.
  ///
  /// For [`InternalPid`], this returns `self`. For [`ExternalPid`], this
  /// extracts the local PID component, discarding node information.
  fn into_internal(self) -> InternalPid;

  /// Converts this identifier into an external PID representation.
  ///
  /// Returns [`Some`] for [`ExternalPid`] and [`None`] for [`InternalPid`].
  fn into_external(self) -> Option<ExternalPid>;
}
