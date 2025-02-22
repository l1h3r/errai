use alloc::borrow::Cow;

use crate::decode::Decode;
use crate::decode::Decoder;
use crate::error::Result;
use crate::frame::DynFrame;
use crate::types::Slice;
use crate::types::Version;

// =============================================================================
// Chapter
// =============================================================================

/// Chapter frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Chap<'a> {
  element_identifier: Cow<'a, str>,
  timestamps: ChapTime,
  #[frame(info = "embedded sub-frames")]
  sub_frames: Cow<'a, Slice>,
}

impl Chap<'_> {
  /// Get an iterator over the embedded sub-frames of the frame.
  #[inline]
  pub fn frames(&self) -> ChapIter<'_> {
    ChapIter::new(self.sub_frames())
  }
}

// =============================================================================
// Chapter Times
// =============================================================================

/// Chapter time data.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChapTime {
  start_time: u32,
  start_from: u32,
  end_time: u32,
  end_from: u32,
}

impl ChapTime {
  /// Get the number of milliseconds from the beginning of the file to the start
  /// of the chapter.
  #[inline]
  pub const fn start_time(&self) -> u32 {
    self.start_time
  }

  /// Get the number of bytes from the beginning of the file to the first byte
  /// of the first audio frame in the chapter.
  #[inline]
  pub const fn start_from(&self) -> u32 {
    self.start_from
  }

  /// Get the number of milliseconds from the beginning of the file to the end
  /// of the chapter.
  #[inline]
  pub const fn end_time(&self) -> u32 {
    self.end_time
  }

  /// Get the number of bytes from the beginning of the file to the first byte
  /// of the audio frame following the end of the chapter.
  #[inline]
  pub const fn end_from(&self) -> u32 {
    self.end_from
  }
}

impl Decode<'_> for ChapTime {
  fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
    Ok(Self {
      start_time: decoder.decode()?,
      end_time: decoder.decode()?,
      start_from: decoder.decode()?,
      end_from: decoder.decode()?,
    })
  }
}

copy_into_owned!(ChapTime);

// =============================================================================
// Chap Iterator
// =============================================================================

/// An iterator over the sub-frames of a [`CHAP`][Chap] frame.
#[derive(Clone, Debug)]
pub struct ChapIter<'a> {
  inner: Decoder<'a>,
}

impl<'a> ChapIter<'a> {
  fn new(input: &'a Slice) -> Self {
    Self {
      inner: Decoder::new(input),
    }
  }
}

impl<'a> Iterator for ChapIter<'a> {
  type Item = Result<DynFrame<'a>>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.inner.is_empty() {
      return None;
    }

    // TODO: This should use the same version as original frame.
    let Ok(Some(frame)) = self.inner.decode_frame(Version::ID3v24) else {
      return self.inner.decode_frame(Version::ID3v23).transpose();
    };

    Some(Ok(frame))
  }
}
