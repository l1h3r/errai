//! Library Errors

use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;
use core::num::TryFromIntError;
use std::error::Error as StdError;
use std::io::Error as IoError;

/// Alias for [`core::result::Result<T, E>`].
pub type Result<T, E = Error> = ::core::result::Result<T, E>;

// =============================================================================
// Error
// =============================================================================

/// Error returned from ID3 operations.
#[derive(Debug)]
pub struct Error {
  kind: ErrorKind,
  base: ErrorBase,
}

impl Error {
  pub(crate) const fn new(kind: ErrorKind) -> Self {
    Self {
      kind,
      base: ErrorBase::Ignore,
    }
  }

  pub(crate) fn new_std(kind: ErrorKind, source: impl StdError + 'static) -> Self {
    Self {
      kind,
      base: ErrorBase::Source(Box::new(source)),
    }
  }

  pub(crate) const fn tag(field: TagField) -> Self {
    Self::new(ErrorKind::InvalidField(field))
  }

  /// Get the category of the error.
  #[inline]
  pub const fn kind(&self) -> ErrorKind {
    self.kind
  }
}

impl Display for Error {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    write!(f, "Error")
  }
}

impl StdError for Error {
  #[inline]
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    match self.base {
      ErrorBase::Ignore => None,
      ErrorBase::Source(ref inner) => Some(&**inner),
    }
  }
}

impl From<IoError> for Error {
  #[inline]
  fn from(other: IoError) -> Self {
    Self::new_std(ErrorKind::IO, other)
  }
}

impl From<TryFromIntError> for Error {
  #[inline]
  fn from(other: TryFromIntError) -> Self {
    Self::new_std(ErrorKind::Int, other)
  }
}

// =============================================================================
// Error Kind
// =============================================================================

/// The types of errors that may occur.
#[derive(Clone, Copy, Debug)]
pub enum ErrorKind {
  /// I/O error.
  IO,
  /// Integer conversion error.
  Int,
  /// Invalid ID3 tag field.
  InvalidField(TagField),
  /// Invalid ID3 tag version.
  InvalidVersion,
  /// Invalid bytes in frame identifier.
  InvalidFrameId,
  /// Invalid bitflag in frame header.
  InvalidBitFlag,
  /// Invalid data found in frame.
  InvalidFrameData,
}

// =============================================================================
// Tag Field
// =============================================================================

/// Fields of an ID3v2 tag.
#[derive(Clone, Copy, Debug)]
pub enum TagField {
  /// Header identifier.
  Identifier,
  /// Header version.
  Version,
  /// Extended header size.
  ExtSize,
  /// Extended header bitflag size.
  ExtFlagSize,
  /// Extended header flag data size.
  ExtFlagData,
}

// =============================================================================
// Error Base
// =============================================================================

#[derive(Debug)]
enum ErrorBase {
  Ignore,
  Source(Box<dyn StdError + 'static>),
}

impl Display for ErrorBase {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    match self {
      Self::Ignore => Ok(()),
      Self::Source(inner) => Display::fmt(inner, f),
    }
  }
}
