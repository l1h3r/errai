use alloc::borrow::Borrow;
use alloc::borrow::Cow;

use crate::decode::Decode;
use crate::decode::Decoder;
use crate::decode::Encoding;
use crate::decode::Language;
use crate::decode::Timestamp;
use crate::error::Error;
use crate::error::ErrorKind;
use crate::error::Result;
use crate::types::Slice;

// =============================================================================
// Synchronised Lyrics
// =============================================================================

/// Synchronised lyrics frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Sylt<'a> {
  text_encoding: Encoding,
  language: Language,
  time_format: Timestamp,
  content_type: ContentType,
  content_descriptor: Cow<'a, str>,
  binary_data: Cow<'a, Slice>,
}

impl Sylt<'_> {
  /// Get an iterator over the lyrics of the frame.
  #[inline]
  pub fn lyrics(&self) -> SyltIter<'_> {
    SyltIter::new(self.text_encoding, self.binary_data())
  }
}

// =============================================================================
// Content Type
// =============================================================================

/// Content type.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ContentType {
  /// other.
  Other = 0x00,
  /// lyrics.
  Lyrics = 0x01,
  /// text transcription.
  Text = 0x02,
  /// movement/part name (e.g. "Adagio").
  Movement = 0x03,
  /// events (e.g. "Don Quijote enters the stage").
  Events = 0x04,
  /// chord (e.g. "Bb F Fsus").
  Chord = 0x05,
  /// trivia/'pop up' information.
  Trivia = 0x06,
}

impl Decode<'_> for ContentType {
  fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
    match u8::decode(decoder)? {
      0x00 => Ok(Self::Other),
      0x01 => Ok(Self::Lyrics),
      0x02 => Ok(Self::Text),
      0x03 => Ok(Self::Movement),
      0x04 => Ok(Self::Events),
      0x05 => Ok(Self::Chord),
      0x06 => Ok(Self::Trivia),
      _ => Err(Error::new(ErrorKind::InvalidFrameData)),
    }
  }
}

copy_into_owned!(ContentType);

// =============================================================================
// Lyric
// =============================================================================

/// Parsed lyric information from a [`SYLT`][Sylt] frame.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Lyric<'a> {
  data: Cow<'a, str>,
  time: u32,
}

impl<'a> Lyric<'a> {
  /// Get the text content of the lyric.
  #[inline]
  pub fn data(&self) -> &str {
    self.data.borrow()
  }

  /// Get the timestamp of the event.
  #[inline]
  pub const fn time(&self) -> u32 {
    self.time
  }
}

impl<'a> Decode<'a> for Lyric<'a> {
  fn decode(decoder: &mut Decoder<'a>) -> Result<Self> {
    Ok(Self {
      data: decoder.decode()?,
      time: decoder.decode()?,
    })
  }
}

// =============================================================================
// Sylt Iterator
// =============================================================================

/// An iterator over the lyrics of a [`SYLT`][Sylt] frame.
#[derive(Clone, Debug)]
pub struct SyltIter<'a> {
  inner: Decoder<'a>,
}

impl<'a> SyltIter<'a> {
  fn new(format: Encoding, input: &'a Slice) -> Self {
    Self {
      inner: Decoder::with_format(input, format),
    }
  }
}

impl<'a> Iterator for SyltIter<'a> {
  type Item = Result<Lyric<'a>>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.inner.is_empty() {
      Some(self.inner.decode())
    } else {
      None
    }
  }
}
