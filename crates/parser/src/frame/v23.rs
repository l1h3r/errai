use bitflags::bitflags;
use core::num::NonZeroU32;
use std::io::Cursor;

use crate::error::Result;
use crate::traits::ReadExt;
use crate::types::FrameId;
use crate::types::Slice;
use crate::types::Version;

// =============================================================================
// Frame - ID3v2.3
// =============================================================================

/// A parsed ID3v2.3 frame.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameV3<'a> {
  identifier: FrameId,
  descriptor: NonZeroU32,
  flag_bytes: FrameV3Flags,
  extra_data: FrameV3Extra,
  frame_data: &'a Slice,
}

impl<'a> FrameV3<'a> {
  /// The size of the frame header (in bytes).
  pub const SIZE: usize = 10;

  /// The version of the frame.
  pub const VERSION: Version = Version::ID3v23;

  /// Get the frame identifier.
  #[inline]
  pub const fn identifier(&self) -> FrameId {
    self.identifier
  }

  /// Get the frame identifier as a string slice.
  #[inline]
  pub const fn identifier_str(&self) -> &str {
    self.identifier.as_str()
  }

  /// Get the frame identifier as a slice of bytes.
  #[inline]
  pub const fn identifier_slice(&self) -> &[u8] {
    self.identifier.as_slice()
  }

  /// Get the size descriptor of the frame content (in bytes).
  #[inline]
  pub const fn descriptor(&self) -> u32 {
    self.descriptor.get()
  }

  /// Get the frame bitflags.
  #[inline]
  pub const fn flag_bytes(&self) -> FrameV3Flags {
    self.flag_bytes
  }

  /// Get the extra data specified by the frame flags.
  #[inline]
  pub const fn extra_data(&self) -> &FrameV3Extra {
    &self.extra_data
  }

  /// Get the raw frame content.
  #[inline]
  pub const fn frame_data(&self) -> &'a Slice {
    self.frame_data
  }

  /// Get the total size of the frame (in bytes).
  #[inline]
  pub const fn total_size(&self) -> usize {
    Self::SIZE + self.descriptor() as usize
  }

  /// Parse an ID3v2.3 frame from the given `slice`.
  pub fn from_slice(slice: &'a Slice) -> Result<Option<Self>> {
    let mut reader: Cursor<&Slice> = slice.cursor();

    let identifier: FrameId = reader.read_array()?.try_into()?;
    let descriptor: NonZeroU32 = reader.read_u32()?.try_into()?;
    let flag_bytes: FrameV3Flags = FrameV3Flags::from_reader(&mut reader)?;
    let extra_data: FrameV3Extra = FrameV3Extra::from_reader(flag_bytes, &mut reader)?;

    let frame_data: &Slice = reader
      .get_ref()
      .skip(Self::SIZE + extra_data.size())
      .take(descriptor.get() as usize - extra_data.size());

    Ok(Some(Self {
      identifier,
      descriptor,
      flag_bytes,
      extra_data,
      frame_data,
    }))
  }
}

// =============================================================================
// Frame Flags
// =============================================================================

bitflags! {
  /// ID3v2.3 frame flags.
  #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
  pub struct FrameV3Flags: u16 {
    /// Tag alter preservation.
    ///
    /// Tells the tag parser what to do with this frame if it is unknown and the
    /// tag is altered in any way.
    const TAG_ALTER_PRESERVATION = 0b10000000_00000000; // 0x8000
    /// File alter preservation.
    ///
    /// Tells the tag parser what to do with this frame if it is unknown and the
    /// file, excluding the tag, is altered.
    const FILE_ALTER_PRESERVATION = 0b01000000_00000000; // 0x4000
    /// Read only.
    ///
    /// Tells the software that the contents of this frame are intended to be
    /// read only.
    ///
    /// Changing the contents might break something, e.g. a signature.
    const READ_ONLY = 0b00100000_00000000; // 0x2000
    /// Compression.
    ///
    /// Indicates whether or not the frame is compressed.
    const COMPRESSION = 0b00000000_10000000; // 0x0080
    /// Encryption.
    ///
    /// Indicates whether or not the frame is enrypted.
    const ENCRYPTION = 0b00000000_01000000; // 0x0040
    /// Grouping identity.
    ///
    /// Indicates whether or not this frame belongs in a group with other frames.
    const GROUPING_IDENTITY = 0b00000000_00100000; // 0x0020
  }
}

impl FrameV3Flags {
  fn from_reader<R>(reader: &mut R) -> Result<Self>
  where
    R: ReadExt,
  {
    reader
      .read_u16()
      .map_err(Into::into)
      .map(Self::from_bits_retain)
  }
}

// =============================================================================
// Frame Extra Data
// =============================================================================

/// Extra frame data specified by bitflags.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameV3Extra {
  comp: Option<u32>, // The decompressed size when `COMPRESSION` is set.
  encr: Option<u8>,  // The method of encryption when `ENCRYPTION` is set.
  grid: Option<u8>,  // The group identifier when `GROUPING_IDENTITY` is set.
}

impl FrameV3Extra {
  /// Get the size of the extra frame data (in bytes).
  #[inline]
  pub const fn size(&self) -> usize {
    let comp_size: usize = self.comp.is_some() as usize * 4;
    let encr_size: usize = self.encr.is_some() as usize;
    let grid_size: usize = self.grid.is_some() as usize;

    comp_size + encr_size + grid_size
  }

  /// Get the size of the decompressed frame content.
  #[inline]
  pub const fn comp(&self) -> Option<u32> {
    self.comp
  }

  /// Get the encryption method of the frame content.
  #[inline]
  pub const fn encr(&self) -> Option<u8> {
    self.encr
  }

  /// Get the group identifier of the frame content.
  #[inline]
  pub const fn grid(&self) -> Option<u8> {
    self.grid
  }

  fn from_reader<R>(bitflags: FrameV3Flags, reader: &mut R) -> Result<Self>
  where
    R: ReadExt,
  {
    let mut this: Self = Self {
      comp: None,
      encr: None,
      grid: None,
    };

    if bitflags.contains(FrameV3Flags::COMPRESSION) {
      this.comp = Some(reader.read_u32()?);
    }

    if bitflags.contains(FrameV3Flags::ENCRYPTION) {
      this.encr = Some(reader.read_u8()?);
    }

    if bitflags.contains(FrameV3Flags::GROUPING_IDENTITY) {
      this.grid = Some(reader.read_u8()?);
    }

    Ok(this)
  }
}
