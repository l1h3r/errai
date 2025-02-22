use core::fmt::Debug;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;

use crate::content::Content;
use crate::error::Error;
use crate::error::ErrorKind;
use crate::error::Result;
use crate::frame::FrameV2;
use crate::frame::FrameV3;
use crate::frame::FrameV4;
use crate::types::Slice;
use crate::types::Version;

// =============================================================================
// Dynamic Frame
// =============================================================================

/// A parsed ID3v2 frame.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum DynFrame<'a> {
  /// ID3v2.2 frame.
  V2(FrameV2<'a>),
  /// ID3v2.3 frame.
  V3(FrameV3<'a>),
  /// ID3v2.4 frame.
  V4(FrameV4<'a>),
}

impl<'a> DynFrame<'a> {
  /// Get the size of the frame header (in bytes).
  #[inline]
  pub const fn header_size(&self) -> usize {
    match self {
      Self::V2(_) => FrameV2::SIZE,
      Self::V3(_) => FrameV3::SIZE,
      Self::V4(_) => FrameV4::SIZE,
    }
  }

  /// Get the version of the frame.
  #[inline]
  pub const fn version(&self) -> Version {
    match self {
      Self::V2(_) => FrameV2::VERSION,
      Self::V3(_) => FrameV3::VERSION,
      Self::V4(_) => FrameV4::VERSION,
    }
  }

  /// Get the frame identifier as a string slice.
  #[inline]
  pub const fn identifier_str(&self) -> &str {
    match self {
      Self::V2(inner) => inner.identifier_str(),
      Self::V3(inner) => inner.identifier_str(),
      Self::V4(inner) => inner.identifier_str(),
    }
  }

  /// Get the frame identifier as a slice of bytes.
  #[inline]
  pub const fn identifier_slice(&self) -> &[u8] {
    match self {
      Self::V2(inner) => inner.identifier_slice(),
      Self::V3(inner) => inner.identifier_slice(),
      Self::V4(inner) => inner.identifier_slice(),
    }
  }

  /// Get the size descriptor of the frame content (in bytes).
  #[inline]
  pub const fn descriptor(&self) -> u32 {
    match self {
      Self::V2(inner) => inner.descriptor(),
      Self::V3(inner) => inner.descriptor(),
      Self::V4(inner) => inner.descriptor(),
    }
  }

  /// Get the raw frame bitflags.
  #[inline]
  pub const fn flag_bytes(&self) -> Option<u16> {
    match self {
      Self::V2(_) => None,
      Self::V3(inner) => Some(inner.flag_bytes().bits()),
      Self::V4(inner) => Some(inner.flag_bytes().bits()),
    }
  }

  /// Get the raw frame content.
  #[inline]
  pub const fn frame_data(&self) -> &'a Slice {
    match self {
      Self::V2(inner) => inner.frame_data(),
      Self::V3(inner) => inner.frame_data(),
      Self::V4(inner) => inner.frame_data(),
    }
  }

  /// Get the total size of the frame (in bytes).
  #[inline]
  pub const fn total_size(&self) -> usize {
    match self {
      Self::V2(inner) => inner.total_size(),
      Self::V3(inner) => inner.total_size(),
      Self::V4(inner) => inner.total_size(),
    }
  }

  /// Decode the contents of the frame.
  #[inline]
  pub fn decode(&self) -> Result<Content<'a>> {
    match self {
      Self::V2(inner) => inner.decode(),
      Self::V3(inner) => inner.decode(),
      Self::V4(inner) => inner.decode(),
    }
  }

  /// Parse an ID3v2 frame from the given `slice`.
  pub fn from_slice(version: Version, slice: &'a Slice) -> Result<Option<Self>> {
    match version {
      Version::ID3v11 => Err(Error::new(ErrorKind::InvalidVersion)),
      Version::ID3v12 => Err(Error::new(ErrorKind::InvalidVersion)),
      FrameV2::VERSION => FrameV2::from_slice(slice).map(|frame| frame.map(Self::V2)),
      FrameV3::VERSION => FrameV3::from_slice(slice).map(|frame| frame.map(Self::V3)),
      FrameV4::VERSION => FrameV4::from_slice(slice).map(|frame| frame.map(Self::V4)),
    }
  }
}

impl Debug for DynFrame<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    match self {
      Self::V2(inner) => Debug::fmt(inner, f),
      Self::V3(inner) => Debug::fmt(inner, f),
      Self::V4(inner) => Debug::fmt(inner, f),
    }
  }
}
