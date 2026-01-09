use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

/// A static literal term.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Atom {
  data: &'static str,
}

impl Atom {
  /// The atom `undefined`.
  pub const UNDEFINED: Self = Self::new("undefined");

  /// Creates a new `Atom`.
  #[inline]
  pub const fn new(data: &'static str) -> Self {
    Self { data }
  }
}

impl Debug for Atom {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    f.write_str(self.data)
  }
}

impl Display for Atom {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    f.write_str(self.data)
  }
}

impl From<&'static str> for Atom {
  #[inline]
  fn from(other: &'static str) -> Self {
    Self::new(other)
  }
}
