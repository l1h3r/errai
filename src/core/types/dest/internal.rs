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
/// let pid = InternalPid::EXAMPLE_PROC;
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
  /// let pid = InternalPid::EXAMPLE_PROC;
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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use hashbrown::HashSet;

  use crate::core::Atom;
  use crate::core::InternalDest;
  use crate::core::InternalPid;

  #[test]
  fn test_from_pid() {
    let data: InternalPid = InternalPid::from_bits(0x123);
    let dest: InternalDest = InternalDest::from(data);

    assert!(
      matches!(dest, InternalDest::Proc(inner) if inner == data),
      "Should be Proc variant"
    );
  }

  #[test]
  fn test_from_atom() {
    let data: Atom = Atom::new("logger");
    let dest: InternalDest = InternalDest::from(data);

    assert!(
      matches!(dest, InternalDest::Name(inner) if inner == data),
      "Should be Name variant"
    );
  }

  #[test]
  fn test_is_proc() {
    let proc: InternalDest = InternalDest::from(InternalPid::from_bits(1));
    let name: InternalDest = InternalDest::from(Atom::new("test"));

    assert!(proc.is_proc());
    assert!(!name.is_proc());
  }

  #[test]
  fn test_is_name() {
    let proc: InternalDest = InternalDest::from(InternalPid::from_bits(1));
    let name: InternalDest = InternalDest::from(Atom::new("test"));

    assert!(!proc.is_name());
    assert!(name.is_name());
  }

  #[test]
  fn test_clone() {
    let src: InternalDest = InternalDest::from(Atom::new("test"));
    let dst = src.clone();

    if let (InternalDest::Name(atom1), InternalDest::Name(atom2)) = (src, dst) {
      assert_eq!(atom1, atom2);
    } else {
      panic!("Both should be Name variant");
    }
  }

  #[test]
  fn test_copy() {
    let src: InternalDest = InternalDest::from(Atom::new("test"));
    let dst: InternalDest = src;

    if let (InternalDest::Name(atom1), InternalDest::Name(atom2)) = (src, dst) {
      assert_eq!(atom1, atom2);
    } else {
      panic!("Both should be Name variant");
    }
  }

  #[test]
  fn test_display_proc() {
    let src: InternalDest = InternalDest::from(InternalPid::from_bits(123));
    let fmt: String = format!("{src}");

    assert!(fmt.starts_with("#PID<"));
    assert!(fmt.ends_with(">"));
  }

  #[test]
  fn test_display_name() {
    let src: InternalDest = InternalDest::from(Atom::new("logger"));
    let fmt: String = format!("{src}");

    assert_eq!(fmt, "logger");
  }

  #[test]
  fn test_debug_equals_display() {
    let src: InternalDest = InternalDest::from(Atom::new("test"));
    let fmt: String = format!("{src}");

    assert_eq!(fmt, format!("{src:?}"));
  }

  #[test]
  fn test_equality() {
    let a: InternalDest = InternalDest::from(InternalPid::from_bits(100));
    let b: InternalDest = InternalDest::from(InternalPid::from_bits(100));
    let c: InternalDest = InternalDest::from(InternalPid::from_bits(200));

    assert_eq!(a, b);
    assert_ne!(a, c);
  }

  #[test]
  fn test_ordering() {
    let a: InternalDest = InternalDest::from(InternalPid::from_bits(100));
    let b: InternalDest = InternalDest::from(InternalPid::from_bits(200));

    assert!(a < b);
  }

  #[test]
  fn test_hash() {
    let mut set: HashSet<InternalDest> = HashSet::new();

    set.insert(InternalDest::from(InternalPid::from_bits(1)));
    set.insert(InternalDest::from(Atom::new("test")));
    set.insert(InternalDest::from(InternalPid::from_bits(1)));

    assert_eq!(set.len(), 2);
  }
}
