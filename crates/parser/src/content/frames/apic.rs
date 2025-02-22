use alloc::borrow::Cow;

use crate::decode::Decode;
use crate::decode::Decoder;
use crate::decode::Encoding;
use crate::error::Error;
use crate::error::ErrorKind;
use crate::error::Result;
use crate::types::Slice;

// =============================================================================
// Attached Picture
// =============================================================================

/// Attached picture frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
#[frame(skip_decoding)]
pub struct Apic<'a> {
  text_encoding: Encoding,
  image_format: ImgType,
  picture_type: PicType,
  description: Cow<'a, str>,
  picture_data: Cow<'a, Slice>,
}

impl<'a> Decode<'a> for Apic<'a> {
  fn decode(decoder: &mut Decoder<'a>) -> Result<Self> {
    Ok(Self {
      text_encoding: decoder.decode()?,
      image_format: decoder.decode()?,
      picture_type: decoder.decode()?,
      description: decoder.decode()?,
      picture_data: decoder.decode()?,
    })
  }

  fn decode_v2(decoder: &mut Decoder<'a>) -> Result<Self> {
    Ok(Self {
      text_encoding: decoder.decode_v2()?,
      image_format: decoder.decode_v2()?,
      picture_type: decoder.decode_v2()?,
      description: decoder.decode_v2()?,
      picture_data: decoder.decode_v2()?,
    })
  }
}

// =============================================================================
// Image Format/MIME Type
// =============================================================================

// TODO: Support image/bmp (?)
// TODO: Support image/gif (?)
// TODO: Support image/tiff (?)

/// Image format.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImgType {
  /// PNG image format.
  Png,
  /// JPG image format.
  Jpg,
}

impl ImgType {
  const PNG: [u8; 3] = *b"PNG";
  const JPG: [u8; 3] = *b"JPG";

  const MIME_PNG: &'static [u8] = b"image/png";
  const MIME_JPG: &'static [u8] = b"image/jpg";
  const MIME_JPEG: &'static [u8] = b"image/jpeg";
}

impl Decode<'_> for ImgType {
  fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
    match decoder.decode_latin1()?.as_bytes() {
      Self::MIME_PNG => Ok(Self::Png),
      Self::MIME_JPG => Ok(Self::Jpg),
      Self::MIME_JPEG => Ok(Self::Jpg),
      _ => Err(Error::new(ErrorKind::InvalidFrameData)),
    }
  }

  fn decode_v2(decoder: &mut Decoder<'_>) -> Result<Self> {
    match decoder.decode()? {
      Self::PNG => Ok(Self::Png),
      Self::JPG => Ok(Self::Jpg),
      _ => Err(Error::new(ErrorKind::InvalidFrameData)),
    }
  }
}

copy_into_owned!(ImgType);

// =============================================================================
// Pic Type
// =============================================================================

/// Picture type.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum PicType {
  /// Other.
  Other = 0x00,
  /// 32x32 pixels 'file icon' (PNG only).
  FileIcon = 0x01,
  /// Other file icon.
  FileIcon2 = 0x02,
  /// Cover (front).
  CoverFront = 0x03,
  /// Cover (back).
  CoverBack = 0x04,
  /// Leaflet page.
  Leaflet = 0x05,
  /// Media (e.g. lable side of CD).
  Media = 0x06,
  /// Lead artist/lead performer/soloist.
  LeadArtist = 0x07,
  /// Artist/performer.
  Artist = 0x08,
  /// Conductor.
  Conductor = 0x09,
  /// Band/Orchestra.
  Band = 0x0A,
  /// Composer.
  Composer = 0x0B,
  /// Lyricist/text writer.
  Lyricist = 0x0C,
  /// Recording Location.
  RecordingLocation = 0x0D,
  /// During recording.
  DuringRecording = 0x0E,
  /// During performance.
  DuringPerformance = 0x0F,
  /// Movie/video screen capture.
  Movie = 0x10,
  /// A bright coloured fish.
  BrightColouredFish = 0x11,
  /// Illustration.
  Illustration = 0x12,
  /// Band/artist logotype.
  BandLogo = 0x13,
  /// Publisher/Studio logotype.
  Publisher = 0x14,
}

impl Decode<'_> for PicType {
  fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
    match u8::decode(decoder)? {
      0x00 => Ok(Self::Other),
      0x01 => Ok(Self::FileIcon),
      0x02 => Ok(Self::FileIcon2),
      0x03 => Ok(Self::CoverFront),
      0x04 => Ok(Self::CoverBack),
      0x05 => Ok(Self::Leaflet),
      0x06 => Ok(Self::Media),
      0x07 => Ok(Self::LeadArtist),
      0x08 => Ok(Self::Artist),
      0x09 => Ok(Self::Conductor),
      0x0A => Ok(Self::Band),
      0x0B => Ok(Self::Composer),
      0x0C => Ok(Self::Lyricist),
      0x0D => Ok(Self::RecordingLocation),
      0x0E => Ok(Self::DuringRecording),
      0x0F => Ok(Self::DuringPerformance),
      0x10 => Ok(Self::Movie),
      0x11 => Ok(Self::BrightColouredFish),
      0x12 => Ok(Self::Illustration),
      0x13 => Ok(Self::BandLogo),
      0x14 => Ok(Self::Publisher),
      _ => Err(Error::new(ErrorKind::InvalidFrameData)),
    }
  }
}

copy_into_owned!(PicType);
