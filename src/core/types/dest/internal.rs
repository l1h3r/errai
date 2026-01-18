use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::core::Atom;
use crate::core::InternalPid;

/// Destination addressing an internal process or registered name.
///
/// This type is used when the target is known to be local, avoiding the
/// overhead of node checks in the send path. The destination resolves to
/// either a direct process identifier or a registered name that will be
/// looked up at send time.
///
/// # Variants
///
/// - [`Proc`]: Direct addressing via process identifier
/// - [`Name`]: Indirect addressing via registered name
///
/// # Name Resolution
///
/// Name-based destinations are resolved when the message is sent. If the
/// name is not registered, the send operation will fail.
///
/// # Examples
///
/// ```
/// use errai::core::{Atom, InternalDest, InternalPid};
///
/// // Direct addressing
/// let pid = InternalPid::from_bits(0x123);
/// let dest = InternalDest::from(pid);
/// assert!(dest.is_proc());
///
/// // Name-based addressing
/// let dest = InternalDest::from(Atom::new("logger"));
/// assert!(dest.is_name());
/// ```
///
/// [`Proc`]: InternalDest::Proc
/// [`Name`]: InternalDest::Name
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum InternalDest {
  /// Destination identifying a local process.
  ///
  /// Messages sent to this destination are delivered directly to the
  /// process identified by the PID.
  Proc(InternalPid),
  /// Destination identifying a locally registered name.
  ///
  /// Messages sent to this destination are delivered to whichever process
  /// currently holds the registered name. The name is resolved at send time.
  Name(Atom),
}

impl InternalDest {
  /// Returns `true` if this destination identifies a process.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::{InternalDest, InternalPid};
  ///
  /// let pid = InternalPid::from_bits(0x123);
  /// let dest = InternalDest::from(pid);
  /// assert!(dest.is_proc());
  /// ```
  #[inline]
  pub const fn is_proc(&self) -> bool {
    matches!(self, Self::Proc(_))
  }

  /// Returns `true` if this destination identifies a registered name.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::{Atom, InternalDest};
  ///
  /// let dest = InternalDest::from(Atom::new("logger"));
  /// assert!(dest.is_name());
  /// ```
  #[inline]
  pub const fn is_name(&self) -> bool {
    matches!(self, Self::Name(_))
  }
}

impl Debug for InternalDest {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Proc(inner) => Debug::fmt(inner, f),
      Self::Name(inner) => Debug::fmt(inner, f),
    }
  }
}

impl Display for InternalDest {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Proc(inner) => Display::fmt(inner, f),
      Self::Name(inner) => Display::fmt(inner, f),
    }
  }
}

impl From<InternalPid> for InternalDest {
  #[inline]
  fn from(other: InternalPid) -> Self {
    Self::Proc(other)
  }
}

impl From<Atom> for InternalDest {
  #[inline]
  fn from(other: Atom) -> Self {
    Self::Name(other)
  }
}

impl From<&'static str> for InternalDest {
  #[inline]
  fn from(other: &'static str) -> Self {
    Self::Name(Atom::new(other))
  }
}
