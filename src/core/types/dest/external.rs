use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::core::Atom;
use crate::core::ExternalPid;
use crate::core::InternalPid;

/// Destination identifying a process or registered name across nodes.
///
/// This type supports all four addressing modes: local/remote PIDs and
/// local/remote names. It is used in contexts where the destination might
/// be distributed, such as monitoring or cross-node messaging.
///
/// # Variants
///
/// - [`InternalProc`]: Local process by PID
/// - [`InternalName`]: Local registered name
/// - [`ExternalProc`]: Remote process by PID
/// - [`ExternalName`]: Remote registered name (name, node)
///
/// # Examples
///
/// ```
/// use errai::core::{Atom, ExternalDest, ExternalPid, InternalPid};
///
/// // Local process
/// let local = ExternalDest::from(InternalPid::from_bits(0x123));
///
/// // Local name
/// let name = ExternalDest::from(Atom::new("logger"));
///
/// // Remote name
/// let remote = ExternalDest::ExternalName(
///   Atom::new("logger"),
///   Atom::new("node@host")
/// );
/// ```
///
/// [`InternalProc`]: ExternalDest::InternalProc
/// [`InternalName`]: ExternalDest::InternalName
/// [`ExternalProc`]: ExternalDest::ExternalProc
/// [`ExternalName`]: ExternalDest::ExternalName
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternalDest {
  /// Destination identifying a local process.
  ///
  /// Messages are delivered directly to the local process.
  InternalProc(InternalPid),
  /// Destination identifying a locally registered name.
  ///
  /// The name is resolved on the local node at send time.
  InternalName(Atom),
  /// Destination identifying a remote process.
  ///
  /// Messages are routed to the node specified in the PID.
  ExternalProc(ExternalPid),
  /// Destination identifying a registered name on a remote node.
  ///
  /// Format: `(name, node)`. The name is resolved on the remote node.
  ExternalName(Atom, Atom),
}

impl ExternalDest {
  /// Returns `true` if this destination identifies a process.
  ///
  /// This includes both local and remote PIDs.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::{ExternalDest, InternalPid};
  ///
  /// let dest = ExternalDest::from(InternalPid::from_bits(0x123));
  /// assert!(dest.is_proc());
  /// ```
  #[inline]
  pub const fn is_proc(&self) -> bool {
    matches!(self, Self::InternalProc(_) | Self::ExternalProc(_))
  }

  /// Returns `true` if this destination identifies a local process.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::{ExternalDest, InternalPid};
  ///
  /// let dest = ExternalDest::from(InternalPid::from_bits(0x123));
  /// assert!(dest.is_internal_proc());
  /// ```
  #[inline]
  pub const fn is_internal_proc(&self) -> bool {
    matches!(self, Self::InternalProc(_))
  }

  /// Returns `true` if this destination identifies a remote process.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::{Atom, ExternalDest, ExternalPid, InternalPid};
  ///
  /// let pid = ExternalPid::new(
  ///   InternalPid::from_bits(0x123),
  ///   Atom::new("node@host")
  /// );
  /// let dest = ExternalDest::from(pid);
  /// assert!(dest.is_external_proc());
  /// ```
  #[inline]
  pub const fn is_external_proc(&self) -> bool {
    matches!(self, Self::ExternalProc(_))
  }

  /// Returns `true` if this destination identifies a registered name.
  ///
  /// This includes both local and remote names.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::{Atom, ExternalDest};
  ///
  /// let dest = ExternalDest::from(Atom::new("logger"));
  /// assert!(dest.is_name());
  /// ```
  #[inline]
  pub const fn is_name(&self) -> bool {
    matches!(self, Self::InternalName(_) | Self::ExternalName(_, _))
  }

  /// Returns `true` if this destination identifies a local registered name.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::{Atom, ExternalDest};
  ///
  /// let dest = ExternalDest::from(Atom::new("logger"));
  /// assert!(dest.is_internal_name());
  /// ```
  #[inline]
  pub const fn is_internal_name(&self) -> bool {
    matches!(self, Self::InternalName(_))
  }

  /// Returns `true` if this destination identifies a remote registered name.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::{Atom, ExternalDest};
  ///
  /// let dest = ExternalDest::ExternalName(
  ///   Atom::new("logger"),
  ///   Atom::new("node@host")
  /// );
  /// assert!(dest.is_external_name());
  /// ```
  #[inline]
  pub const fn is_external_name(&self) -> bool {
    matches!(self, Self::ExternalName(_, _))
  }
}

impl Debug for ExternalDest {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::InternalProc(inner) => Debug::fmt(inner, f),
      Self::ExternalProc(inner) => Debug::fmt(inner, f),
      Self::InternalName(inner) => Debug::fmt(inner, f),
      Self::ExternalName(inner, _node) => Debug::fmt(inner, f),
    }
  }
}

impl Display for ExternalDest {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::InternalProc(inner) => Display::fmt(inner, f),
      Self::ExternalProc(inner) => Display::fmt(inner, f),
      Self::InternalName(inner) => Display::fmt(inner, f),
      Self::ExternalName(inner, _node) => Display::fmt(inner, f),
    }
  }
}

impl From<InternalPid> for ExternalDest {
  #[inline]
  fn from(other: InternalPid) -> Self {
    Self::InternalProc(other)
  }
}

impl From<ExternalPid> for ExternalDest {
  #[inline]
  fn from(other: ExternalPid) -> Self {
    Self::ExternalProc(other)
  }
}

impl From<Atom> for ExternalDest {
  #[inline]
  fn from(other: Atom) -> Self {
    Self::InternalName(other)
  }
}

impl From<&'static str> for ExternalDest {
  #[inline]
  fn from(other: &'static str) -> Self {
    Self::InternalName(Atom::new(other))
  }
}

impl From<(Atom, Atom)> for ExternalDest {
  #[inline]
  fn from(other: (Atom, Atom)) -> Self {
    Self::ExternalName(other.0, other.1)
  }
}

impl From<(&'static str, &'static str)> for ExternalDest {
  #[inline]
  fn from(other: (&'static str, &'static str)) -> Self {
    Self::ExternalName(Atom::new(other.0), Atom::new(other.1))
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use crate::core::Atom;
  use crate::core::ExternalDest;
  use crate::core::InternalPid;

  #[test]
  fn test_clone() {
    let src: ExternalDest = ExternalDest::from(Atom::new("test"));
    let dst = src.clone();

    if let (ExternalDest::InternalName(atom1), ExternalDest::InternalName(atom2)) = (src, dst) {
      assert_eq!(atom1, atom2);
    } else {
      panic!("Both should be InternalName variant");
    }
  }

  #[test]
  fn test_copy() {
    let src: ExternalDest = ExternalDest::from(Atom::new("test"));
    let dst: ExternalDest = src;

    if let (ExternalDest::InternalName(atom1), ExternalDest::InternalName(atom2)) = (src, dst) {
      assert_eq!(atom1, atom2);
    } else {
      panic!("Both should be InternalName variant");
    }
  }

  #[test]
  fn test_display_proc() {
    let src: ExternalDest = ExternalDest::from(InternalPid::from_bits(123));
    let fmt: String = format!("{src}");

    assert!(fmt.starts_with("#PID<"));
    assert!(fmt.ends_with(">"));
  }

  #[test]
  fn test_display_name() {
    let src: ExternalDest = ExternalDest::from(Atom::new("logger"));
    let fmt: String = format!("{src}");

    assert_eq!(fmt, "logger");
  }

  #[test]
  fn test_debug_equals_display() {
    let src: ExternalDest = ExternalDest::from(Atom::new("test"));
    let fmt: String = format!("{src}");

    assert_eq!(fmt, format!("{src:?}"));
  }
}
