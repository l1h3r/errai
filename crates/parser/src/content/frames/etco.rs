use alloc::borrow::Cow;

use crate::decode::Decode;
use crate::decode::Decoder;
use crate::decode::Timestamp;
use crate::error::Result;
use crate::types::Slice;

// =============================================================================
// Event Timing Codes
// =============================================================================

/// Event timing codes frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Etco<'a> {
  time_format: Timestamp,
  event_codes: Cow<'a, Slice>,
}

impl Etco<'_> {
  /// Get an iterator over the events of the frame.
  #[inline]
  pub fn events(&self) -> EtcoIter<'_> {
    EtcoIter::new(self.event_codes())
  }
}

// =============================================================================
// Event Type
// =============================================================================

/// Event type.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum EventType {
  /// padding (has no meaning).
  Padding = 0x00,
  /// end of initial silence.
  EndSilence = 0x01,
  /// intro start.
  IntroStart = 0x02,
  /// mainpart start.
  MainStart = 0x03,
  /// outro start.
  OutroStart = 0x04,
  /// outro end.
  OutroEnd = 0x05,
  /// verse start.
  VerseStart = 0x06,
  /// refrain start.
  RefrainStart = 0x07,
  /// interlude start.
  InterludeStart = 0x08,
  /// theme start.
  ThemeStart = 0x09,
  /// variation start.
  VariationStart = 0x0A,
  /// key change.
  KeyChange = 0x0B,
  /// time change.
  TimeChange = 0x0C,
  /// momentary unwanted noise (Snap, Crackle & Pop).
  UnwantedNoise = 0x0D,
  /// sustained noise.
  SustainedNoise = 0x0E,
  /// sustained noise end.
  SustainedNoiseEnd = 0x0F,
  /// intro end.
  IntroEnd = 0x10,
  /// mainpart end.
  MainEnd = 0x11,
  /// verse end.
  VerseEnd = 0x12,
  /// refrain end.
  RefrainEnd = 0x13,
  /// theme end.
  ThemeEnd = 0x14,
  // ===========================================================================
  // Reserved (0x15..=0xDF)
  // ===========================================================================
  /// not predefined sync 0
  NotPredefined0 = 0xE0,
  /// not predefined sync 1
  NotPredefined1 = 0xE1,
  /// not predefined sync 2
  NotPredefined2 = 0xE2,
  /// not predefined sync 3
  NotPredefined3 = 0xE3,
  /// not predefined sync 4
  NotPredefined4 = 0xE4,
  /// not predefined sync 5
  NotPredefined5 = 0xE5,
  /// not predefined sync 6
  NotPredefined6 = 0xE6,
  /// not predefined sync 7
  NotPredefined7 = 0xE7,
  /// not predefined sync 8
  NotPredefined8 = 0xE8,
  /// not predefined sync 9
  NotPredefined9 = 0xE9,
  /// not predefined sync A
  NotPredefinedA = 0xEA,
  /// not predefined sync B
  NotPredefinedB = 0xEB,
  /// not predefined sync C
  NotPredefinedC = 0xEC,
  /// not predefined sync D
  NotPredefinedD = 0xED,
  /// not predefined sync E
  NotPredefinedE = 0xEE,
  /// not predefined sync F
  NotPredefinedF = 0xEF,
  // ===========================================================================
  // Reversed (0xF0..=0xFC)
  // ===========================================================================
  /// audio end (start of silence).
  AudioEnd = 0xFD,
  /// audio file ends.
  AudioFileEnd = 0xFE,
  /// one more byte of events follows.
  OneMoreByte = 0xFF,
  /// reversed.
  Reserved = 0x15,
}

impl Decode<'_> for EventType {
  fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
    match u8::decode(decoder)? {
      0x00 => Ok(Self::Padding),
      0x01 => Ok(Self::EndSilence),
      0x02 => Ok(Self::IntroStart),
      0x03 => Ok(Self::MainStart),
      0x04 => Ok(Self::OutroStart),
      0x05 => Ok(Self::OutroEnd),
      0x06 => Ok(Self::VerseStart),
      0x07 => Ok(Self::RefrainStart),
      0x08 => Ok(Self::InterludeStart),
      0x09 => Ok(Self::ThemeStart),
      0x0A => Ok(Self::VariationStart),
      0x0B => Ok(Self::KeyChange),
      0x0C => Ok(Self::TimeChange),
      0x0D => Ok(Self::UnwantedNoise),
      0x0E => Ok(Self::SustainedNoise),
      0x0F => Ok(Self::SustainedNoiseEnd),
      0x10 => Ok(Self::IntroEnd),
      0x11 => Ok(Self::MainEnd),
      0x12 => Ok(Self::VerseEnd),
      0x13 => Ok(Self::RefrainEnd),
      0x14 => Ok(Self::ThemeEnd),
      0x15..=0xDF => Ok(Self::Reserved),
      0xE0 => Ok(Self::NotPredefined0),
      0xE1 => Ok(Self::NotPredefined1),
      0xE2 => Ok(Self::NotPredefined2),
      0xE3 => Ok(Self::NotPredefined3),
      0xE4 => Ok(Self::NotPredefined4),
      0xE5 => Ok(Self::NotPredefined5),
      0xE6 => Ok(Self::NotPredefined6),
      0xE7 => Ok(Self::NotPredefined7),
      0xE8 => Ok(Self::NotPredefined8),
      0xE9 => Ok(Self::NotPredefined9),
      0xEA => Ok(Self::NotPredefinedA),
      0xEB => Ok(Self::NotPredefinedB),
      0xEC => Ok(Self::NotPredefinedC),
      0xED => Ok(Self::NotPredefinedD),
      0xEE => Ok(Self::NotPredefinedE),
      0xEF => Ok(Self::NotPredefinedF),
      0xF0..=0xFC => Ok(Self::Reserved),
      0xFD => Ok(Self::AudioEnd),
      0xFE => Ok(Self::AudioFileEnd),
      0xFF => Ok(Self::OneMoreByte),
    }
  }
}

// =============================================================================
// Event
// =============================================================================

/// Parsed event information from an [`ETCO`][Etco] frame.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventData {
  kind: EventType,
  time: u32,
}

impl EventData {
  /// Get the type of the event.
  #[inline]
  pub const fn kind(&self) -> EventType {
    self.kind
  }

  /// Get the timestamp of the event.
  #[inline]
  pub const fn time(&self) -> u32 {
    self.time
  }
}

impl Decode<'_> for EventData {
  fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
    Ok(Self {
      kind: decoder.decode()?,
      time: decoder.decode()?,
    })
  }
}

// =============================================================================
// Etco Iterator
// =============================================================================

/// An iterator over the events of an [`ETCO`][Etco] frame.
#[derive(Clone, Debug)]
pub struct EtcoIter<'a> {
  inner: Decoder<'a>,
}

impl<'a> EtcoIter<'a> {
  fn new(input: &'a Slice) -> Self {
    Self {
      inner: Decoder::new(input),
    }
  }
}

impl Iterator for EtcoIter<'_> {
  type Item = Result<EventData>;

  fn next(&mut self) -> Option<Self::Item> {
    if !self.inner.is_empty() {
      Some(self.inner.decode())
    } else {
      None
    }
  }
}
