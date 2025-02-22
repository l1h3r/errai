use crate::decode::Decode;
use crate::decode::Decoder;
use crate::error::Error;
use crate::error::ErrorKind;
use crate::error::Result;

// =============================================================================
// Timestamp Format
// =============================================================================

/// Timestamp format.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Timestamp {
  /// Absolute time, 32 bit sized, using MPEG frames as unit.
  MpegFrames = 0x01,
  /// Absolute time, 32 bit sized, using milliseconds as unit.
  Milliseconds = 0x02,
}

impl Decode<'_> for Timestamp {
  fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
    match u8::decode(decoder)? {
      0x01 => Ok(Self::MpegFrames),
      0x02 => Ok(Self::Milliseconds),
      _ => Err(Error::new(ErrorKind::InvalidFrameData)),
    }
  }
}
