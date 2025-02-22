use alloc::borrow::Cow;

use crate::decode::Date;
use crate::decode::Decode;
use crate::decode::Decoder;
use crate::decode::Encoding;
use crate::error::Error;
use crate::error::ErrorKind;
use crate::error::Result;
use crate::types::Slice;

// =============================================================================
// Commercial Frame
// =============================================================================

/// Commercial frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Comr<'a> {
  text_encoding: Encoding,
  #[frame(read = "@latin1")]
  price_string: Cow<'a, str>,
  #[frame(info = "validity date")]
  valid_until: Date,
  #[frame(read = "@latin1")]
  contact_url: Cow<'a, str>,
  #[frame(info = "delivery format")]
  received_as: ReceivedAs,
  seller_name: Cow<'a, str>,
  description: Cow<'a, str>,
  #[frame(read = "@latin1")]
  mime_type: Cow<'a, str>,
  seller_logo: Cow<'a, Slice>,
}

// =============================================================================
// Received As
// =============================================================================

/// Describes how the audio is delivered when purchased.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ReceivedAs {
  /// Other.
  Other = 0x00,
  /// Standard CD album with other songs.
  Standard = 0x01,
  /// Compressed audio on CD.
  Compressed = 0x02,
  /// File over the Internet.
  InternetFile = 0x03,
  /// Stream over the Internet.
  InternetStream = 0x04,
  /// As note sheets.
  NoteSheets = 0x05,
  /// As note sheets in a book with other sheets.
  NoteSheetsBook = 0x06,
  /// Music on other media.
  Music = 0x07,
  /// Non-musical merchandise.
  NonMusical = 0x08,
}

impl Decode<'_> for ReceivedAs {
  fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
    match u8::decode(decoder)? {
      0x00 => Ok(Self::Other),
      0x01 => Ok(Self::Standard),
      0x02 => Ok(Self::Compressed),
      0x03 => Ok(Self::InternetFile),
      0x04 => Ok(Self::InternetStream),
      0x05 => Ok(Self::NoteSheets),
      0x06 => Ok(Self::NoteSheetsBook),
      0x07 => Ok(Self::Music),
      0x08 => Ok(Self::NonMusical),
      _ => Err(Error::new(ErrorKind::InvalidFrameData)),
    }
  }
}

copy_into_owned!(ReceivedAs);
