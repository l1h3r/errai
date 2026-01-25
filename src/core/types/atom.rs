use std::borrow::Cow;
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::LazyLock;

use crate::core::AtomTable;
use crate::core::fatal;
use crate::core::raise;

// -----------------------------------------------------------------------------
// Global Atom Table
// -----------------------------------------------------------------------------

macro_rules! insert_well_known {
  ($table:expr, $value:literal, Atom::$expected:ident) => {{
    let valid: bool = $table
      .insert($value)
      .map(|slot| slot == Atom::$expected.into_slot())
      .unwrap_or_else(|error| fatal!(error));

    if !valid {
      fatal!("invalid well-known atom")
    }
  }};
}

/// Global atom table initialized with well-known runtime atoms.
///
/// This table is lazily initialized on first access and ensures well-known
/// atoms occupy their expected slot indices.
static ATOM_TABLE: LazyLock<AtomTable> = LazyLock::new(|| {
  let table: AtomTable = AtomTable::new();

  insert_well_known!(table, "", Atom::EMPTY);

  insert_well_known!(table, "kill", Atom::KILL);
  insert_well_known!(table, "killed", Atom::KILLED);
  insert_well_known!(table, "normal", Atom::NORMAL);

  insert_well_known!(table, "noproc", Atom::NOPROC);
  insert_well_known!(table, "noconn", Atom::NOCONN);

  insert_well_known!(table, "undefined", Atom::UNDEFINED);

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
  pub const KILL: Self = Self::from_slot(1);

  /// Atom representing the value `killed`.
  pub const KILLED: Self = Self::from_slot(2);

  /// Atom representing the value `normal`.
  pub const NORMAL: Self = Self::from_slot(3);

  /// Atom representing the value `noproc`.
  pub const NOPROC: Self = Self::from_slot(4);

  /// Atom representing the value `noconn`.
  pub const NOCONN: Self = Self::from_slot(5);

  /// Atom representing the value `undefined`.
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
  /// [`MAX_ATOM_BYTES`]: crate::core::MAX_ATOM_BYTES
  /// [`MAX_ATOM_COUNT`]: crate::core::MAX_ATOM_COUNT
  #[inline]
  pub fn new(data: &str) -> Self {
    match ATOM_TABLE.insert(data) {
      Ok(slot) => Self::from_slot(slot),
      Err(error) => raise!(Error, SysCap, error),
    }
  }

  /// Returns the string value associated with this atom.
  ///
  /// This operation is zero-copy and returns a reference to the interned
  /// string with a `'static` lifetime.
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
    match ATOM_TABLE.lookup(self.slot) {
      Ok(data) => data,
      Err(error) => fatal!(error),
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
  #[inline]
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

impl From<Cow<'_, str>> for Atom {
  #[inline]
  fn from(other: Cow<'_, str>) -> Atom {
    Atom::new(other.as_ref())
  }
}

impl From<Box<str>> for Atom {
  #[inline]
  fn from(other: Box<str>) -> Atom {
    Atom::new(other.as_ref())
  }
}

impl From<Rc<str>> for Atom {
  #[inline]
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

impl From<Atom> for Cow<'static, str> {
  #[inline]
  fn from(other: Atom) -> Self {
    Cow::Borrowed(other.as_str())
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

// -----------------------------------------------------------------------------
// Extensions - PartialEq
// -----------------------------------------------------------------------------

macro_rules! impl_partial_eq {
  ($type:ty, $convert_that:path) => {
    impl_partial_eq!($type, Atom::as_str, $convert_that);
  };
  ($type:ty, $convert_atom:path, $convert_that:path) => {
    impl PartialEq<$type> for Atom {
      #[inline]
      fn eq(&self, other: &$type) -> bool {
        $convert_atom(self) == $convert_that(other)
      }
    }

    impl PartialEq<&$type> for Atom {
      #[inline]
      fn eq(&self, other: &&$type) -> bool {
        $convert_atom(self) == $convert_that(other)
      }
    }

    impl PartialEq<Atom> for $type {
      #[inline]
      fn eq(&self, other: &Atom) -> bool {
        $convert_that(self) == $convert_atom(other)
      }
    }

    impl PartialEq<Atom> for &$type {
      #[inline]
      fn eq(&self, other: &Atom) -> bool {
        $convert_that(self) == $convert_atom(other)
      }
    }
  };
}

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

impl PartialEq<&str> for Atom {
  #[inline]
  fn eq(&self, other: &&str) -> bool {
    self.as_str() == *other
  }
}

impl PartialEq<Atom> for &str {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    *self == other.as_str()
  }
}

impl PartialEq<&&str> for Atom {
  #[inline]
  fn eq(&self, other: &&&str) -> bool {
    self.as_str() == **other
  }
}

impl PartialEq<Atom> for &&str {
  #[inline]
  fn eq(&self, other: &Atom) -> bool {
    **self == other.as_str()
  }
}

impl_partial_eq!(String, String::as_str);

impl_partial_eq!(Cow<'_, str>, AsRef::as_ref);

impl_partial_eq!(Box<str>, AsRef::as_ref);
impl_partial_eq!(Rc<str>, AsRef::as_ref);
impl_partial_eq!(Arc<str>, AsRef::as_ref);

impl_partial_eq!(Path, Path::as_os_str);
impl_partial_eq!(PathBuf, Path::as_os_str);

impl_partial_eq!(OsStr, OsStr::new, OsStrExt::as_os_str);
impl_partial_eq!(OsString, OsString::as_os_str);

// TODO: Replace this trait with `OsStr::as_os_str` once `str_as_str` is stable.
//
// https://doc.rust-lang.org/nightly/std/ffi/os_str/struct.OsStr.html#method.as_os_str
// https://github.com/rust-lang/rust/issues/130366
trait OsStrExt {
  fn as_os_str(&self) -> &OsStr;
}

impl OsStrExt for OsStr {
  #[inline]
  fn as_os_str(&self) -> &OsStr {
    self
  }
}

impl OsStrExt for &OsStr {
  #[inline]
  fn as_os_str(&self) -> &OsStr {
    self
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::borrow::Cow;
  use std::collections::HashSet;
  use std::ffi::OsStr;
  use std::ffi::OsString;
  use std::ops::Deref;
  use std::path::Path;
  use std::path::PathBuf;
  use std::rc::Rc;
  use std::sync::Arc;

  use crate::core::Atom;

  macro_rules! assert_equality {
    ($atom:expr, $cmp1:expr, $cmp2:expr) => {{
      ::std::assert_eq!($atom, $cmp1);
      ::std::assert_ne!($atom, $cmp2);
      ::std::assert_eq!($atom, &$cmp1);
      ::std::assert_ne!($atom, &$cmp2);

      ::std::assert_eq!($cmp1, $atom);
      ::std::assert_ne!($cmp2, $atom);
      ::std::assert_eq!(&$cmp1, $atom);
      ::std::assert_ne!(&$cmp2, $atom);
    }};
  }

  #[test]
  fn test_well_known_atoms_have_correct_values() {
    assert_eq!(Atom::EMPTY.as_str(), "");
    assert_eq!(Atom::KILL.as_str(), "kill");
    assert_eq!(Atom::KILLED.as_str(), "killed");
    assert_eq!(Atom::NORMAL.as_str(), "normal");
    assert_eq!(Atom::NOPROC.as_str(), "noproc");
    assert_eq!(Atom::NOCONN.as_str(), "noconn");
    assert_eq!(Atom::UNDEFINED.as_str(), "undefined");
  }

  #[test]
  fn test_new() {
    assert_eq!(Atom::new("test").as_str(), "test");
  }

  #[test]
  fn test_new_empty_string() {
    assert_eq!(Atom::new("").as_str(), "");
    assert_eq!(Atom::new(""), Atom::EMPTY);
  }

  #[test]
  fn test_new_unicode() {
    assert_eq!(Atom::new("こんにちは").as_str(), "こんにちは");
  }

  #[test]
  fn test_new_emoji() {
    assert_eq!(Atom::new("🦀").as_str(), "🦀");
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
    let src: Atom = Atom::new("test");
    let fmt: String = format!("{src}");

    assert_eq!(fmt, "test");
  }

  #[test]
  fn test_debug_equals_display() {
    let src: Atom = Atom::new("test");
    let fmt: String = format!("{src}");

    assert_eq!(fmt, format!("{src:?}"));
  }

  #[test]
  fn test_default() {
    assert_eq!(<Atom as Default>::default(), Atom::EMPTY);
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
  fn test_ordering() {
    let a: Atom = Atom::new("apple");
    let b: Atom = Atom::new("banana");
    let c: Atom = Atom::new("cherry");

    assert!(a < b);
    assert!(b < c);
    assert!(a < c);
  }

  #[test]
  fn test_deref() {
    let atom: Atom = Atom::new("test");
    let data: &str = Deref::deref(&atom);

    assert_eq!(atom.as_str(), data);
  }

  #[test]
  fn test_as_ref() {
    let atom: Atom = Atom::new("test");
    let data: &str = AsRef::as_ref(&atom);
    assert_eq!(atom.as_str(), data);

    let data: &OsStr = AsRef::as_ref(&atom);
    assert_eq!(atom.as_str(), data);
  }

  #[test]
  fn test_hash() {
    let mut set: HashSet<Atom> = HashSet::new();

    set.insert(Atom::new("a"));
    set.insert(Atom::new("b"));
    set.insert(Atom::new("a"));

    assert_eq!(set.len(), 2);
    assert!(set.contains(&Atom::new("a")));
    assert!(set.contains(&Atom::new("b")));
  }

  #[test]
  fn test_from_into_str() {
    let atom: Atom = Atom::from("test");
    let data: &str = atom.into();

    assert_eq!(data, "test");
    assert_eq!(atom, Atom::from(data));
  }

  #[test]
  fn test_from_into_string() {
    let atom: Atom = Atom::from("test");
    let data: String = atom.into();

    assert_eq!(data, "test");
    assert_eq!(atom, Atom::from(data));
  }

  #[test]
  fn test_from_into_cow() {
    let atom: Atom = Atom::from("test");
    let data: Cow<'_, str> = atom.into();

    assert_eq!(data, "test");
    assert_eq!(atom, Atom::from(data));
  }

  #[test]
  fn test_from_into_box() {
    let atom: Atom = Atom::from("test");
    let data: Box<str> = atom.into();

    assert_eq!(data, Box::from("test"));
    assert_eq!(atom, Atom::from(data));
  }

  #[test]
  fn test_from_into_rc() {
    let atom: Atom = Atom::from("test");
    let data: Rc<str> = atom.into();

    assert_eq!(data, Rc::from("test"));
    assert_eq!(atom, Atom::from(data));
  }

  #[test]
  fn test_from_into_arc() {
    let atom: Atom = Atom::from("test");
    let data: Arc<str> = atom.into();

    assert_eq!(data, Arc::from("test"));
    assert_eq!(atom, Atom::from(data));
  }

  #[test]
  fn test_equality_str() {
    let atom: Atom = Atom::new("test");
    let cmp1: &str = "test";
    let cmp2: &str = "other";

    assert_eq!(atom, *cmp1);
    assert_ne!(atom, *cmp2);

    assert_eq!(*cmp1, atom);
    assert_ne!(*cmp2, atom);

    assert_equality!(atom, cmp1, cmp2);
  }

  #[test]
  fn test_equality_string() {
    let atom: Atom = Atom::new("test");
    let cmp1: String = "test".to_owned();
    let cmp2: String = "other".to_owned();

    assert_equality!(atom, cmp1, cmp2);
  }

  #[test]
  fn test_equality_cow_borrowed() {
    let atom: Atom = Atom::new("test");
    let cmp1: Cow<'_, str> = Cow::Borrowed("test");
    let cmp2: Cow<'_, str> = Cow::Borrowed("other");

    assert_equality!(atom, cmp1, cmp2);
  }

  #[test]
  fn test_equality_cow_owned() {
    let atom: Atom = Atom::new("test");
    let cmp1: Cow<'_, str> = Cow::Owned("test".to_owned());
    let cmp2: Cow<'_, str> = Cow::Owned("other".to_owned());

    assert_equality!(atom, cmp1, cmp2);
  }

  #[test]
  fn test_equality_box() {
    let atom: Atom = Atom::new("test");
    let cmp1: Box<str> = Box::from("test");
    let cmp2: Box<str> = Box::from("other");

    assert_equality!(atom, cmp1, cmp2);
  }

  #[test]
  fn test_equality_rc() {
    let atom: Atom = Atom::new("test");
    let cmp1: Rc<str> = Rc::from("test");
    let cmp2: Rc<str> = Rc::from("other");

    assert_equality!(atom, cmp1, cmp2);
  }

  #[test]
  fn test_equality_arc() {
    let atom: Atom = Atom::new("test");
    let cmp1: Arc<str> = Arc::from("test");
    let cmp2: Arc<str> = Arc::from("other");

    assert_equality!(atom, cmp1, cmp2);
  }

  #[test]
  fn test_equality_path() {
    let atom: Atom = Atom::new("test");
    let cmp1: &Path = Path::new("test");
    let cmp2: &Path = Path::new("other");

    assert_equality!(atom, *cmp1, *cmp2);
  }

  #[test]
  fn test_equality_path_buf() {
    let atom: Atom = Atom::new("test");
    let cmp1: PathBuf = PathBuf::from("test");
    let cmp2: PathBuf = PathBuf::from("other");

    assert_equality!(atom, cmp1, cmp2);
  }

  #[test]
  fn test_equality_os_str() {
    let atom: Atom = Atom::new("test");
    let cmp1: &OsStr = OsStr::new("test");
    let cmp2: &OsStr = OsStr::new("other");

    assert_equality!(atom, *cmp1, *cmp2);
  }

  #[test]
  fn test_equality_os_string() {
    let atom: Atom = Atom::new("test");
    let cmp1: OsString = OsString::from("test");
    let cmp2: OsString = OsString::from("other");

    assert_equality!(atom, cmp1, cmp2);
  }
}
