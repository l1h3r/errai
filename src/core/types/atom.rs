//! Atom type providing efficient, interned, immutable string identifiers.
//!
//! This module provides the [`Atom`] type, a lightweight handle to globally
//! interned strings. Atoms enable fast equality comparisons and efficient
//! memory usage for frequently used string values.
//!
//! # Core Properties
//!
//! - **Interned**: Each unique string is stored exactly once
//! - **Immutable**: Atom values cannot be changed after creation
//! - **Fast comparison**: Equality checks compare 32-bit slot indices
//! - **Zero-copy**: Converting to string slices requires no allocation
//!
//! # Well-Known Atoms
//!
//! The runtime pre-allocates several atoms for common values:
//!
//! - [`Atom::EMPTY`]: The empty string `""`
//! - [`Atom::NORMAL`]: Process exit reason `"normal"`
//! - [`Atom::KILL`]: Unconditional kill signal `"kill"`
//! - [`Atom::KILLED`]: Killed exit reason `"killed"`
//! - [`Atom::NOPROC`]: No such process error `"noproc"`
//! - [`Atom::NOCONN`]: No connection error `"noconn"`
//! - [`Atom::UNDEFINED`]: Undefined value `"undefined"`
//!
//! # Examples
//!
//! ```
//! use errai::core::Atom;
//!
//! // Create atoms from strings
//! let hello = Atom::new("hello");
//! let world = Atom::from("world");
//!
//! // Fast equality (compares slot indices, not strings)
//! assert_eq!(Atom::new("test"), Atom::new("test"));
//!
//! // Access string value
//! assert_eq!(hello.as_str(), "hello");
//!
//! // Use well-known atoms
//! assert_eq!(Atom::NORMAL, "normal");
//! ```
//!
//! # Performance Characteristics
//!
//! - **Creation**: O(1) for existing atoms, O(n) for new atoms
//! - **Equality**: O(1) integer comparison
//! - **Ordering**: O(n) string comparison (delegates to underlying string)
//! - **Memory**: 4 bytes per [`Atom`] instance

use std::borrow::Cow;
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::LazyLock;

use crate::core::AtomTable;
use crate::raise;

// -----------------------------------------------------------------------------
// Atom Table
// -----------------------------------------------------------------------------

/// Global atom table initialized with well-known runtime atoms.
///
/// This table is lazily initialized on first access and ensures well-known
/// atoms occupy their expected slot indices.
static ATOM_TABLE: LazyLock<AtomTable> = LazyLock::new(|| {
  let table: AtomTable = AtomTable::new();

  assert_eq!(table.set("").unwrap(), Atom::EMPTY.into_slot());

  assert_eq!(table.set("kill").unwrap(), Atom::KILL.into_slot());
  assert_eq!(table.set("killed").unwrap(), Atom::KILLED.into_slot());
  assert_eq!(table.set("normal").unwrap(), Atom::NORMAL.into_slot());

  assert_eq!(table.set("noproc").unwrap(), Atom::NOPROC.into_slot());
  assert_eq!(table.set("noconn").unwrap(), Atom::NOCONN.into_slot());

  assert_eq!(table.set("undefined").unwrap(), Atom::UNDEFINED.into_slot());

  table
});

// -----------------------------------------------------------------------------
// Atom
// -----------------------------------------------------------------------------

/// Interned, immutable identifier representing a runtime-wide static string.
///
/// Atoms are lightweight handles (32-bit slot indices) to globally interned
/// strings. They provide fast equality comparisons and efficient memory usage
/// for string values that appear multiple times in the system.
///
/// # Memory Layout
///
/// [`Atom`] is a transparent wrapper around a `u32` slot index:
///
/// ```text
/// Atom { slot: u32 }  // 4 bytes
/// ```
///
/// The actual string data lives in the global atom table and is shared
/// across all [`Atom`] instances with the same value.
///
/// # Equality and Ordering
///
/// Equality comparisons are performed on slot indices (O(1)), while ordering
/// comparisons delegate to the underlying string values (O(n)).
///
/// # Examples
///
/// ```
/// use errai::core::Atom;
///
/// let a1 = Atom::new("hello");
/// let a2 = Atom::new("hello");
///
/// assert_eq!(a1, a2);               // Fast: compares slot indices
/// assert_eq!(a1.as_str(), "hello"); // Zero-copy string access
/// ```
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct Atom {
  slot: u32,
}

impl Atom {
  /// Atom representing the empty string.
  pub const EMPTY: Self = Self::from_slot(0);

  /// Atom representing the value `kill`.
  ///
  /// Used for unconditional process termination signals.
  pub const KILL: Self = Self::from_slot(1);

  /// Atom representing the value `killed`.
  ///
  /// Used as an exit reason when a process is killed.
  pub const KILLED: Self = Self::from_slot(2);

  /// Atom representing the value `normal`.
  ///
  /// Used as an exit reason for normal process termination.
  pub const NORMAL: Self = Self::from_slot(3);

  /// Atom representing the value `noproc`.
  ///
  /// Used to indicate that a referenced process does not exist.
  pub const NOPROC: Self = Self::from_slot(4);

  /// Atom representing the value `noconn`.
  ///
  /// Used to indicate that a connection to a remote node does not exist.
  pub const NOCONN: Self = Self::from_slot(5);

  /// Atom representing the value `undefined`.
  ///
  /// Used to represent undefined or uninitialized values.
  pub const UNDEFINED: Self = Self::from_slot(6);

  /// Constructs an atom from a raw atom table slot.
  #[inline]
  pub(crate) const fn from_slot(slot: u32) -> Self {
    Self { slot }
  }

  /// Returns the atom table slot backing this atom.
  #[inline]
  pub(crate) const fn into_slot(self) -> u32 {
    self.slot
  }

  /// Interns a string and returns its corresponding atom.
  ///
  /// If the string has been interned before, returns the existing atom.
  /// Otherwise, allocates a new slot in the global atom table.
  ///
  /// # Panics
  ///
  /// Panics if the string exceeds [`MAX_ATOM_BYTES`] or the atom table
  /// has reached [`MAX_ATOM_COUNT`] capacity.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Atom;
  ///
  /// let atom1 = Atom::new("hello");
  /// let atom2 = Atom::new("hello");
  ///
  /// assert_eq!(atom1, atom2); // Same string, same atom
  /// ```
  ///
  /// [`MAX_ATOM_BYTES`]: crate::consts::MAX_ATOM_BYTES
  /// [`MAX_ATOM_COUNT`]: crate::consts::MAX_ATOM_COUNT
  #[inline]
  pub fn new(data: &str) -> Self {
    match ATOM_TABLE.set(data) {
      Ok(slot) => Self::from_slot(slot),
      Err(error) => raise!(Error, SysCap, error),
    }
  }

  /// Returns the string value associated with this atom.
  ///
  /// This operation is zero-copy and returns a reference to the interned
  /// string with a `'static` lifetime.
  ///
  /// # Panics
  ///
  /// Panics if the atom's slot index is invalid. This should never occur
  /// with atoms constructed through the public API.
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::Atom;
  ///
  /// let atom = Atom::new("example");
  /// assert_eq!(atom.as_str(), "example");
  /// ```
  #[inline]
  pub fn as_str(&self) -> &'static str {
    match ATOM_TABLE.get(self.slot) {
      Ok(data) => data,
      Err(error) => raise!(Error, SysInv, error),
    }
  }
}

impl Debug for Atom {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    Display::fmt(self, f)
  }
}

impl Display for Atom {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    Display::fmt(self.as_str(), f)
  }
}

impl Default for Atom {
  #[inline]
  fn default() -> Self {
    Self::EMPTY
  }
}

impl PartialOrd for Atom {
  #[inline]
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for Atom {
  fn cmp(&self, other: &Self) -> Ordering {
    Ord::cmp(self.as_str(), other.as_str())
  }
}

impl Deref for Atom {
  type Target = str;

  #[inline]
  fn deref(&self) -> &Self::Target {
    self.as_str()
  }
}

impl<T> AsRef<T> for Atom
where
  T: ?Sized,
  str: AsRef<T>,
{
  #[inline]
  fn as_ref(&self) -> &T {
    AsRef::as_ref(self.as_str())
  }
}

// -----------------------------------------------------------------------------
// Extensions - From
// -----------------------------------------------------------------------------

impl From<&str> for Atom {
  #[inline]
  fn from(other: &str) -> Atom {
    Atom::new(other)
  }
}

impl From<String> for Atom {
  #[inline]
  fn from(other: String) -> Atom {
    Atom::new(other.as_str())
  }
}

impl From<&String> for Atom {
  #[inline]
  fn from(other: &String) -> Atom {
    Atom::new(other.as_str())
  }
}

impl From<Box<str>> for Atom {
  #[inline]
  fn from(other: Box<str>) -> Atom {
    Atom::new(other.as_ref())
  }
}

impl From<Rc<str>> for Atom {
  fn from(other: Rc<str>) -> Atom {
    Atom::new(other.as_ref())
  }
}

impl From<Arc<str>> for Atom {
  #[inline]
  fn from(other: Arc<str>) -> Atom {
    Atom::new(other.as_ref())
  }
}

impl From<Cow<'_, str>> for Atom {
  fn from(other: Cow<'_, str>) -> Atom {
    Atom::new(other.as_ref())
  }
}

impl From<Atom> for &'static str {
  #[inline]
  fn from(other: Atom) -> &'static str {
    other.as_str()
  }
}

impl From<Atom> for String {
  #[inline]
  fn from(other: Atom) -> Self {
    String::from(other.as_str())
  }
}

impl From<Atom> for Box<str> {
  #[inline]
  fn from(other: Atom) -> Self {
    Box::from(other.as_str())
  }
}

impl From<Atom> for Rc<str> {
  #[inline]
  fn from(other: Atom) -> Self {
    Rc::from(other.as_str())
  }
}

impl From<Atom> for Arc<str> {
  #[inline]
  fn from(other: Atom) -> Self {
    Arc::from(other.as_str())
  }
}

impl From<Atom> for Cow<'static, str> {
  #[inline]
  fn from(other: Atom) -> Self {
    Cow::Borrowed(other.as_str())
  }
}

// -----------------------------------------------------------------------------
// Extensions - PartialEq
// -----------------------------------------------------------------------------

impl PartialEq<str> for Atom {
  #[inline]
  fn eq(&self, other: &str) -> bool {
    self.as_str() == other
  }
}

impl PartialEq<Atom> for str {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self == other.as_str()
  }
}

impl PartialEq<Cow<'_, str>> for Atom {
  #[inline]
  fn eq(&self, other: &Cow<'_, str>) -> bool {
    self.as_str() == other.as_ref()
  }
}

impl PartialEq<&str> for Atom {
  #[inline]
  fn eq(&self, other: &&str) -> bool {
    self.as_str() == *other
  }
}

impl PartialEq<&&str> for Atom {
  #[inline]
  fn eq(&self, other: &&&str) -> bool {
    self.as_str() == **other
  }
}

impl PartialEq<String> for Atom {
  #[inline]
  fn eq(&self, other: &String) -> bool {
    self.as_str() == other
  }
}

impl PartialEq<&String> for Atom {
  #[inline]
  fn eq(&self, other: &&String) -> bool {
    self.as_str() == *other
  }
}

impl PartialEq<Box<str>> for Atom {
  #[inline]
  fn eq(&self, other: &Box<str>) -> bool {
    self.as_str() == other.as_ref()
  }
}

impl PartialEq<Rc<str>> for Atom {
  #[inline]
  fn eq(&self, other: &Rc<str>) -> bool {
    self.as_str() == other.as_ref()
  }
}

impl PartialEq<Arc<str>> for Atom {
  #[inline]
  fn eq(&self, other: &Arc<str>) -> bool {
    self.as_str() == other.as_ref()
  }
}

impl PartialEq<&Cow<'_, str>> for Atom {
  #[inline]
  fn eq(&self, other: &&Cow<'_, str>) -> bool {
    self.as_str() == other.as_ref()
  }
}

impl PartialEq<Atom> for &str {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    *self == other.as_str()
  }
}

impl PartialEq<Atom> for &&str {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    **self == other.as_str()
  }
}

impl PartialEq<Atom> for String {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self == other.as_str()
  }
}

impl PartialEq<Atom> for &String {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    *self == other.as_str()
  }
}

impl PartialEq<Atom> for Box<str> {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self.as_ref() == other.as_str()
  }
}

impl PartialEq<Atom> for &Box<str> {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self.as_ref() == other.as_str()
  }
}

impl PartialEq<Atom> for Rc<str> {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self.as_ref() == other.as_str()
  }
}

impl PartialEq<Atom> for &Rc<str> {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self.as_ref() == other.as_str()
  }
}

impl PartialEq<Atom> for Arc<str> {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self.as_ref() == other.as_str()
  }
}

impl PartialEq<Atom> for &Arc<str> {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self.as_ref() == other.as_str()
  }
}

impl PartialEq<Atom> for Cow<'_, str> {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self.as_ref() == other.as_str()
  }
}

impl PartialEq<Atom> for &Cow<'_, str> {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self.as_ref() == other.as_str()
  }
}

impl PartialEq<Atom> for Path {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self == Path::new(other)
  }
}

impl PartialEq<Atom> for &Path {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    *self == Path::new(other)
  }
}

impl PartialEq<Atom> for OsStr {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    self == OsStr::new(other)
  }
}

impl PartialEq<Atom> for &OsStr {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    *self == OsStr::new(other)
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use hashbrown::HashSet;

  use crate::core::Atom;

  // #[test]
  // fn test_well_known_atoms_have_correct_values() {
  //   assert_eq!(Atom::EMPTY.as_str(), "");
  //   assert_eq!(Atom::NORMAL.as_str(), "normal");
  //   assert_eq!(Atom::KILL.as_str(), "kill");
  //   assert_eq!(Atom::KILLED.as_str(), "killed");
  //   assert_eq!(Atom::NOPROC.as_str(), "noproc");
  //   assert_eq!(Atom::NOCONN.as_str(), "noconn");
  //   assert_eq!(Atom::UNDEFINED.as_str(), "undefined");
  // }

  #[test]
  fn test_new() {
    let atom: Atom = Atom::new("test");
    assert_eq!(atom.as_str(), "test");
  }

  #[test]
  fn test_new_empty_string() {
    let atom: Atom = Atom::new("");
    assert_eq!(atom.as_str(), "");
    assert_eq!(atom, Atom::EMPTY);
  }

  #[test]
  fn test_new_unicode() {
    let atom: Atom = Atom::new("こんにちは");
    assert_eq!(atom.as_str(), "こんにちは");
  }

  #[test]
  fn test_new_emoji() {
    let atom: Atom = Atom::new("🦀");
    assert_eq!(atom.as_str(), "🦀");
  }

  #[test]
  #[should_panic]
  fn test_new_too_long() {
    Atom::new(&"🦀".repeat(256));
  }

  #[test]
  fn test_interning() {
    let a: Atom = Atom::new("hello");
    let b: Atom = Atom::new("hello");
    let c: Atom = Atom::new("world");

    assert_eq!(a.into_slot(), b.into_slot());
    assert_ne!(a.into_slot(), c.into_slot());
  }

  #[test]
  fn test_from_str() {
    let atom: Atom = Atom::from("test");
    assert_eq!(atom.as_str(), "test");
  }

  #[test]
  fn test_from_string() {
    let atom: Atom = Atom::from("test".to_string());
    assert_eq!(atom.as_str(), "test");
  }

  #[test]
  fn test_clone() {
    let src: Atom = Atom::new("test");
    let dst: Atom = src.clone();

    assert_eq!(src, dst);
  }

  #[test]
  fn test_copy() {
    let src: Atom = Atom::new("test");
    let dst: Atom = src;

    assert_eq!(src, dst);
  }

  #[test]
  fn test_display() {
    let src: Atom = Atom::new("display_test");
    let fmt: String = format!("{src}");

    assert_eq!(fmt, "display_test");
  }

  #[test]
  fn test_debug_equals_display() {
    let src: Atom = Atom::new("debug_test");
    let fmt: String = format!("{src}");

    assert_eq!(fmt, format!("{src:?}"));
  }

  #[test]
  fn test_equality() {
    let a: Atom = Atom::new("foo");
    let b: Atom = Atom::new("foo");
    let c: Atom = Atom::new("bar");

    assert_eq!(a, b);
    assert_ne!(a, c);
  }

  #[test]
  fn test_equality_with_str() {
    let atom: Atom = Atom::new("test");

    assert_eq!(atom, "test");
    assert_ne!(atom, "other");
  }

  #[test]
  fn test_ordering() {
    let a: Atom = Atom::new("apple");
    let b: Atom = Atom::new("banana");
    let c: Atom = Atom::new("cherry");

    assert!(a < b);
    assert!(b < c);
    assert!(a < c);
  }

  #[test]
  fn test_hash() {
    let mut set: HashSet<Atom> = HashSet::new();

    set.insert(Atom::new("one"));
    set.insert(Atom::new("two"));
    set.insert(Atom::new("one"));

    assert_eq!(set.len(), 2);
    assert!(set.contains(&Atom::new("one")));
    assert!(set.contains(&Atom::new("two")));
  }
}
