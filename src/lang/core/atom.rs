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

use crate::erts::Runtime;

/// A static literal term.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct Atom {
  slot: u32,
}

impl Atom {
  pub const EMPTY: Self = Self::from_slot(0);

  pub const KILL: Self = Self::from_slot(1);
  pub const KILLED: Self = Self::from_slot(2);
  pub const NORMAL: Self = Self::from_slot(3);

  pub const NOPROC: Self = Self::from_slot(4);
  pub const NOCONN: Self = Self::from_slot(5);

  /// Creates a new `Atom` from an atom table index.
  #[inline]
  pub(crate) const fn from_slot(slot: u32) -> Self {
    Self { slot }
  }

  /// Returns the index of the atom in the atom table.
  #[inline]
  pub(crate) const fn into_slot(self) -> u32 {
    self.slot
  }

  /// Creates a new `Atom` from the given `data`.
  #[inline]
  pub fn new(data: &str) -> Self {
    Self::from_slot(Runtime::set_atom(data))
  }

  #[inline]
  pub fn as_str(&self) -> &'static str {
    Runtime::get_atom(self.slot)
  }
}

impl Debug for Atom {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    Debug::fmt(self.as_str(), f)
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
