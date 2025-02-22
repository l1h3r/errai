use bitflags::bitflags;

use crate::error::Error;
use crate::error::Result;
use crate::error::TagField;
use crate::traits::ReadExt;

// =============================================================================
// Extended Header
// =============================================================================

/// A parsed ID3v2 extended header.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExtHeader {
  ext_size: u32,
  bitflags: ExtHeaderFlags,
  pad_size: u32,
  crc_data: Option<u32>,
  restrict: Option<Restrictions>,
}

impl ExtHeader {
  /// Get the extended header size (in bytes).
  #[inline]
  pub const fn ext_size(&self) -> u32 {
    self.ext_size
  }

  /// Get the extended header bitflags.
  #[inline]
  pub const fn bitflags(&self) -> ExtHeaderFlags {
    self.bitflags
  }

  /// Get the padding size (in bytes).
  #[inline]
  pub const fn pad_size(&self) -> u32 {
    self.pad_size
  }

  /// Get the extended header CRC-32 data.
  #[inline]
  pub const fn crc_data(&self) -> Option<u32> {
    self.crc_data
  }

  /// Get the extended header tag restrictions.
  #[inline]
  pub const fn restrictions(&self) -> Option<Restrictions> {
    self.restrict
  }

  /// Returns `true` if the `CRC_DATA_PRESENT` flag is set (and applicable).
  #[inline]
  pub const fn flag_crc(&self) -> bool {
    match self.bitflags {
      ExtHeaderFlags::V3(inner) => inner.contains(ExtHeaderFlagsV3::CRC_DATA_PRESENT),
      ExtHeaderFlags::V4(inner) => inner.contains(ExtHeaderFlagsV4::CRC_DATA_PRESENT),
    }
  }

  /// Returns `true` if the `TAG_IS_UPDATE` flag is set (and applicable).
  #[inline]
  pub const fn flag_update(&self) -> bool {
    match self.bitflags {
      ExtHeaderFlags::V3(_) => false,
      ExtHeaderFlags::V4(inner) => inner.contains(ExtHeaderFlagsV4::TAG_IS_UPDATE),
    }
  }

  /// Returns `true` if the `TAG_RESTRICTIONS` flag is set (and applicable).
  #[inline]
  pub const fn flag_restrictions(&self) -> bool {
    match self.bitflags {
      ExtHeaderFlags::V3(_) => false,
      ExtHeaderFlags::V4(inner) => inner.contains(ExtHeaderFlagsV4::TAG_RESTRICTIONS),
    }
  }

  /// Parse an ID3v2.3 extended header from the given `reader`.
  pub fn from_reader_v3<R>(mut reader: R) -> Result<Self>
  where
    R: ReadExt,
  {
    // Simple 32-bit unsigned integer
    let ext_size: u32 = reader.read_u32()?;

    // Must be 6 or 10 bytes
    if !(ext_size == 0x06 || ext_size == 0x0A) {
      return Err(Error::tag(TagField::ExtSize));
    }

    // 2 bytes - only 1 valid flag value.
    let bitflags: ExtHeaderFlags = ExtHeaderFlags::from_reader_v3(&mut reader)?;

    let mut this: Self = Self {
      ext_size,
      bitflags,
      pad_size: reader.read_u32()?,
      crc_data: None,
      restrict: None,
    };

    // If the CRC flag is set, read the 4-byte value.
    if this.flag_crc() {
      this.crc_data = Some(reader.read_u32()?);
    }

    Ok(this)
  }

  /// Parse an ID3v2.4 extended header from the given `reader`.
  pub fn from_reader_v4<R>(mut reader: R) -> Result<Self>
  where
    R: ReadExt,
  {
    // 28-bit "unsynchronized" integer.
    let ext_size: u32 = reader.read_u28_unsync()?;

    // Must be between 6 and 15 bytes.
    if !(0x06..=0x0F).contains(&ext_size) {
      return Err(Error::tag(TagField::ExtSize));
    }

    // Number of flag bytes - must be "1"
    if reader.read_u8()? != 0x01 {
      return Err(Error::tag(TagField::ExtFlagSize));
    }

    // 1 byte for extended header flags.
    let bitflags: ExtHeaderFlags = ExtHeaderFlags::from_reader_v4(&mut reader)?;

    let mut this: Self = Self {
      ext_size,
      bitflags,
      pad_size: 0,
      crc_data: None,
      restrict: None,
    };

    if this.flag_update() {
      // Flag data length must be 0x00.
      if reader.read_u8()? != 0x00 {
        return Err(Error::tag(TagField::ExtFlagData));
      }
    }

    if this.flag_crc() {
      // Flag data length must be 0x05.
      if reader.read_u8()? != 0x05 {
        return Err(Error::tag(TagField::ExtFlagData));
      }

      // 35-bit "unsynchronized" integer.
      this.crc_data = Some(reader.read_u35_unsync()?);
    }

    if this.flag_restrictions() {
      // Flag data length must be 0x01.
      if reader.read_u8()? != 0x01 {
        return Err(Error::tag(TagField::ExtFlagData));
      }

      this.restrict = Some(Restrictions::from_reader(&mut reader)?);
    }

    Ok(this)
  }
}

// =============================================================================
// Extended Header Flags
// =============================================================================

/// ID3v2 extended header flags
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtHeaderFlags {
  /// ID3v2.3 flags.
  V3(ExtHeaderFlagsV3),
  /// ID3v2.4 flags.
  V4(ExtHeaderFlagsV4),
}

impl ExtHeaderFlags {
  fn from_reader_v3<R>(reader: &mut R) -> Result<Self>
  where
    R: ReadExt,
  {
    reader
      .read_u16()
      .map_err(Into::into)
      .map(ExtHeaderFlagsV3::from_bits_retain)
      .map(Self::V3)
  }

  fn from_reader_v4<R>(reader: &mut R) -> Result<Self>
  where
    R: ReadExt,
  {
    reader
      .read_u8()
      .map_err(Into::into)
      .map(ExtHeaderFlagsV4::from_bits_retain)
      .map(Self::V4)
  }
}

bitflags! {
  /// ID3v2 extended header flags (ID3v2.3).
  #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
  pub struct ExtHeaderFlagsV3: u16 {
    /// CRC data present.
    ///
    /// Indicates that a 4-byte CRC-32 data is appended to the extended header.
    const CRC_DATA_PRESENT = 0b10000000_00000000;
  }

  /// ID3v2 extended header flags (ID3v2.4).
  #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
  pub struct ExtHeaderFlagsV4: u8 {
    /// Tag is an update.
    ///
    /// The tag is an update of a tag found earlier in the file or stream.
    const TAG_IS_UPDATE = 0b01000000;
    /// CRC data present.
    ///
    /// CRC-32 (ISO-3309) data is appended to the extended header.
    const CRC_DATA_PRESENT = 0b00100000;
    /// Tag restrictions.
    ///
    /// Additional tag restrictions.
    const TAG_RESTRICTIONS = 0b00010000;
  }
}

// =============================================================================
// Tag Restrictions
// =============================================================================

/// Additional restrictions applicable to the encoding process.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Restrictions {
  tag_size: TagSizeRestriction,
  text_enc: TextEncRestriction,
  text_len: TextLenRestriction,
  image_enc: ImageEncRestriction,
  image_len: ImageLenRestriction,
}

impl Restrictions {
  /// Get the tag size restrictions.
  #[inline]
  pub const fn tag_size(&self) -> TagSizeRestriction {
    self.tag_size
  }

  /// Get the text encoding restrictions.
  #[inline]
  pub const fn text_enc(&self) -> TextEncRestriction {
    self.text_enc
  }

  /// Get the text field size restrictions.
  #[inline]
  pub const fn text_len(&self) -> TextLenRestriction {
    self.text_len
  }

  /// Get the image encoding restrictions.
  #[inline]
  pub const fn image_enc(&self) -> ImageEncRestriction {
    self.image_enc
  }

  /// Get the image size restrictions.
  #[inline]
  pub const fn image_len(&self) -> ImageLenRestriction {
    self.image_len
  }

  fn from_reader<R>(reader: &mut R) -> Result<Self>
  where
    R: ReadExt,
  {
    let byte: u8 = reader.read_u8()?;

    Ok(Self {
      tag_size: TagSizeRestriction::from_u8(byte),
      text_enc: TextEncRestriction::from_u8(byte),
      text_len: TextLenRestriction::from_u8(byte),
      image_enc: ImageEncRestriction::from_u8(byte),
      image_len: ImageLenRestriction::from_u8(byte),
    })
  }
}

/// Tag size restrictions.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TagSizeRestriction {
  /// No more than 128 frames and 1 MB total tag size.
  R1 = 0b00000000,
  /// No more than 64 frames and 128 KB total tag size.
  R2 = 0b01000000,
  /// No more than 32 frames and 40 KB total tag size.
  R3 = 0b10000000,
  /// No more than 32 frames and 4 KB total tag size.
  R4 = 0b11000000,
}

impl TagSizeRestriction {
  /// The bitmask applicable to these restrictions.
  pub const MASK: u8 = 0b11000000;

  const fn from_u8(value: u8) -> Self {
    match value & Self::MASK {
      0b00000000 => Self::R1,
      0b01000000 => Self::R2,
      0b10000000 => Self::R3,
      0b11000000 => Self::R4,
      _ => unreachable!(),
    }
  }
}

/// Text encoding restrictions.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TextEncRestriction {
  /// No restrictions.
  None = 0b00000000,
  /// Strings are only encoded with ISO-8859-1 or UTF-8.
  Some = 0b00100000,
}

impl TextEncRestriction {
  /// The bitmask applicable to these restrictions.
  pub const MASK: u8 = 0b00100000;

  const fn from_u8(value: u8) -> Self {
    match value & Self::MASK {
      0b00000000 => Self::None,
      0b00100000 => Self::Some,
      _ => unreachable!(),
    }
  }
}

/// Text field size restrictions.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum TextLenRestriction {
  /// No restrictions.
  R1 = 0b00000000,
  /// No string is longer than 1024 characters.
  R2 = 0b00001000,
  /// No string is longer than 128 characters.
  R3 = 0b00010000,
  /// No string is longer than 30 characters.
  R4 = 0b00011000,
}

impl TextLenRestriction {
  /// The bitmask applicable to these restrictions.
  pub const MASK: u8 = 0b00011000;

  const fn from_u8(value: u8) -> Self {
    match value & Self::MASK {
      0b00000000 => Self::R1,
      0b00001000 => Self::R2,
      0b00010000 => Self::R3,
      0b00011000 => Self::R4,
      _ => unreachable!(),
    }
  }
}

/// Image encoding restrictions.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ImageEncRestriction {
  /// No restrictions.
  None = 0b00000000,
  /// Images are encoded only with PNG or JPEG.
  Some = 0b00000100,
}

impl ImageEncRestriction {
  /// The bitmask applicable to these restrictions.
  pub const MASK: u8 = 0b00000100;

  const fn from_u8(value: u8) -> Self {
    match value & Self::MASK {
      0b00000000 => Self::None,
      0b00000100 => Self::Some,
      _ => unreachable!(),
    }
  }
}

/// Image size restrictions.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum ImageLenRestriction {
  /// No restrictions
  R1 = 0b00000000,
  /// All images are 256x256 pixels or smaller.
  R2 = 0b00000001,
  /// All images are 64x64 pixels or smaller.
  R3 = 0b00000010,
  /// All images are exactly 64x64 pixels, unless required otherwise.
  R4 = 0b00000011,
}

impl ImageLenRestriction {
  /// The bitmask applicable to these restrictions.
  pub const MASK: u8 = 0b00000011;

  const fn from_u8(value: u8) -> Self {
    match value & Self::MASK {
      0b00000000 => Self::R1,
      0b00000001 => Self::R2,
      0b00000010 => Self::R3,
      0b00000011 => Self::R4,
      _ => unreachable!(),
    }
  }
}
