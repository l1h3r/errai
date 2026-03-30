use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

use crate::core::Atom;
use crate::core::LocalPid;

/// Destination addressing a local process or registered name.
///
/// # Name Resolution
///
/// Name-based destinations are resolved when the message is sent. If the
/// name is not registered, the send operation will fail.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum LocalDest {
  /// Destination identifying a local process.
  Proc(LocalPid),
  /// Destination identifying a locally registered name.
  Name(Atom),
}

impl LocalDest {
  /// Returns `true` if this destination identifies a process.
  #[inline]
  pub const fn is_proc(&self) -> bool {
    matches!(self, Self::Proc(_))
  }

  /// Returns `true` if this destination identifies a registered name.
  #[inline]
  pub const fn is_name(&self) -> bool {
    matches!(self, Self::Name(_))
  }
}

impl Debug for LocalDest {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Proc(inner) => Debug::fmt(inner, f),
      Self::Name(inner) => Debug::fmt(inner, f),
    }
  }
}

impl Display for LocalDest {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    match self {
      Self::Proc(inner) => Display::fmt(inner, f),
      Self::Name(inner) => Display::fmt(inner, f),
    }
  }
}

impl From<LocalPid> for LocalDest {
  #[inline]
  fn from(other: LocalPid) -> Self {
    Self::Proc(other)
  }
}

impl From<Atom> for LocalDest {
  #[inline]
  fn from(other: Atom) -> Self {
    Self::Name(other)
  }
}

impl From<&'static str> for LocalDest {
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
  use crate::core::Atom;
  use crate::core::LocalDest;
  use crate::core::LocalPid;

  #[test]
  fn test_is_proc() {
    let name: LocalDest = LocalDest::Name(Atom::new("test"));
    let proc: LocalDest = LocalDest::Proc(LocalPid::ROOT_PROC);

    assert!(proc.is_proc());
    assert!(!name.is_proc());
  }

  #[test]
  fn test_is_name() {
    let name: LocalDest = LocalDest::Name(Atom::new("test"));
    let proc: LocalDest = LocalDest::Proc(LocalPid::ROOT_PROC);

    assert!(name.is_name());
    assert!(!proc.is_name());
  }

  #[test]
  fn test_clone() {
    let src: LocalDest = LocalDest::Proc(LocalPid::ROOT_PROC);
    let dst: LocalDest = src.clone();

    assert_eq!(src, dst);
  }

  #[test]
  fn test_copy() {
    let src: LocalDest = LocalDest::Proc(LocalPid::ROOT_PROC);
    let dst: LocalDest = src;

    assert_eq!(src, dst);
  }

  #[test]
  fn test_display_transparent() {
    let src: LocalPid = LocalPid::ROOT_PROC;
    let dst: LocalDest = LocalDest::Proc(src);

    assert_eq!(format!("{dst}"), format!("{src}"));
  }

  #[test]
  fn test_debug_transparent() {
    let src: LocalPid = LocalPid::ROOT_PROC;
    let dst: LocalDest = LocalDest::Proc(src);

    assert_eq!(format!("{dst:?}"), format!("{src:?}"));
  }

  #[test]
  fn test_from_pid() {
    let src: LocalPid = LocalPid::ROOT_PROC;
    let dst: LocalDest = LocalDest::from(src);

    assert_eq!(dst, LocalDest::Proc(src));
  }

  #[test]
  fn test_from_atom() {
    let src: Atom = Atom::new("test");
    let dst: LocalDest = LocalDest::from(src);

    assert_eq!(dst, LocalDest::Name(src));
  }

  #[test]
  fn test_from_static_str() {
    let src: &str = "test";
    let dst: LocalDest = LocalDest::from(src);

    assert_eq!(dst, LocalDest::Name(Atom::new("test")));
  }
}
