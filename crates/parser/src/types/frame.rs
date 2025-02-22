use core::fmt::Debug;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;
use core::ops::Deref;
use core::str::from_utf8_unchecked;

use crate::error::Error;
use crate::error::ErrorKind;
use crate::utils;

// =============================================================================
// Frame Identifier
// =============================================================================

/// An ID3 frame identifier.
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameId<const S: usize = 4> {
  inner: [u8; S],
}

impl<const S: usize> FrameId<S> {
  /// Create a new `FrameId` with no safety checks.
  ///
  /// # Safety
  ///
  /// Caller is responsible for ensuring the given bytes form a valid frame
  /// identifier. See [`utils::is_frame_id`] for more information.
  pub const unsafe fn new_unchecked(inner: [u8; S]) -> Self {
    debug_assert!(utils::is_frame_id(inner.as_slice()));
    Self { inner }
  }

  /// Get a string representation of the frame ID.
  #[inline]
  pub const fn as_str(&self) -> &str {
    // SAFETY: We validated the input bytes on creation
    unsafe { from_utf8_unchecked(self.as_slice()) }
  }

  /// Get a shared reference to the underlying array of bytes.
  #[inline]
  pub const fn as_array(&self) -> &[u8; S] {
    &self.inner
  }

  /// Get a shared reference to the underlying slice of bytes.
  #[inline]
  pub const fn as_slice(&self) -> &[u8] {
    &self.inner
  }
}

impl<const S: usize> Debug for FrameId<S> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    Debug::fmt(self.as_str(), f)
  }
}

impl<const S: usize> Display for FrameId<S> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    Display::fmt(self.as_str(), f)
  }
}

impl<const S: usize> Deref for FrameId<S> {
  type Target = str;

  #[inline]
  fn deref(&self) -> &Self::Target {
    self.as_str()
  }
}

impl<const S: usize> TryFrom<[u8; S]> for FrameId<S> {
  type Error = Error;

  fn try_from(other: [u8; S]) -> Result<Self, Self::Error> {
    if utils::is_frame_id(other.as_slice()) {
      // SAFETY: We just ensured the validity of the input bytes.
      Ok(unsafe { Self::new_unchecked(other) })
    } else {
      Err(Error::new(ErrorKind::InvalidFrameId))
    }
  }
}
