use bitflags::bitflags;
use core::num::NonZeroU32;
use std::io::Cursor;

use crate::error::Error;
use crate::error::ErrorKind;
use crate::error::Result;
use crate::traits::ReadExt;
use crate::types::FrameId;
use crate::types::Slice;
use crate::types::Version;

// =============================================================================
// Frame - ID3v2.4
// =============================================================================

/// A parsed ID3v2.4 frame.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameV4<'a> {
  identifier: FrameId,
  descriptor: NonZeroU32,
  flag_bytes: FrameV4Flags,
  extra_data: FrameV4Extra,
  frame_data: &'a Slice,
}

impl<'a> FrameV4<'a> {
  /// The size of the frame header (in bytes).
  pub const SIZE: usize = 10;

  /// The version of the frame.
  pub const VERSION: Version = Version::ID3v24;

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
  pub const fn flag_bytes(&self) -> FrameV4Flags {
    self.flag_bytes
  }

  /// Get the extra data specified by the frame flags.
  #[inline]
  pub const fn extra_data(&self) -> &FrameV4Extra {
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

  /// Parse an ID3v2.4 frame from the given `slice`.
  pub fn from_slice(slice: &'a Slice) -> Result<Option<Self>> {
    let mut reader: Cursor<&Slice> = slice.cursor();

    let identifier: FrameId = reader.read_array()?.try_into()?;
    let descriptor: NonZeroU32 = reader.read_u28_unsync()?.try_into()?;
    let flag_bytes: FrameV4Flags = FrameV4Flags::from_reader(&mut reader)?;
    let extra_data: FrameV4Extra = FrameV4Extra::from_reader(flag_bytes, &mut reader)?;

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
  /// ID3v2.4 frame flags.
  #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
  pub struct FrameV4Flags: u16 {
    /// Tag alter preservation.
    ///
    /// Tells the tag parser what to do with this frame if it is unknown and the
    /// tag is altered in any way.
    const TAG_ALTER_PRESERVATION = 0b01000000_00000000; // 0x4000
    /// File alter preservation.
    ///
    /// Tells the tag parser what to do with this frame if it is unknown and the
    /// file, excluding the tag, is altered.
    const FILE_ALTER_PRESERVATION = 0b00100000_00000000; // 0x2000
    /// Read only.
    ///
    /// Tells the software that the contents of this frame are intended to be
    /// read only.
    ///
    /// Changing the contents might break something, e.g. a signature.
    const READ_ONLY = 0b00010000_00000000; // 0x1000
    /// Grouping identity.
    ///
    /// Indicates whether or not this frame belongs in a group with other frames.
    const GROUPING_IDENTITY = 0b00000000_01000000; // 0x0040
    /// Compression.
    ///
    /// Indicates whether or not the frame is compressed.
    const COMPRESSION = 0b00000000_00001000; // 0x0008
    /// Encryption.
    ///
    /// Indicates whether or not the frame is encrypted.
    const ENCRYPTION = 0b00000000_00000100; // 0x0004
    /// Unsynchronisation.
    ///
    /// Indicates whether or not unsynchronisation was applied to this frame.
    const UNSYNCHRONISATION = 0b00000000_00000010; // 0x0002
    /// Data length indicator.
    ///
    /// Indicates that a data length indicator has been added to the frame.
    const DATA_LENGTH_INDICATOR = 0b00000000_00000001; // 0x0001
  }
}

impl FrameV4Flags {
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
pub struct FrameV4Extra {
  grid: Option<u8>,  // The group identifier when `GROUPING_IDENTITY` is set.
  encr: Option<u8>,  // The method of encryption when `ENCRYPTION` is set.
  dlen: Option<u32>, // The frame length when `DATA_LENGTH_INDICATOR` is set.
}

impl FrameV4Extra {
  /// Get the size of the extra frame data (in bytes).
  #[inline]
  pub const fn size(&self) -> usize {
    let grid_size: usize = self.grid.is_some() as usize;
    let encr_size: usize = self.encr.is_some() as usize;
    let dlen_size: usize = self.dlen.is_some() as usize * 4;

    grid_size + encr_size + dlen_size
  }

  /// Get the group identifier of the frame content.
  #[inline]
  pub const fn grid(&self) -> Option<u8> {
    self.grid
  }

  /// Get the encryption method of the frame content.
  #[inline]
  pub const fn encr(&self) -> Option<u8> {
    self.encr
  }

  /// Get the data length indicator of the frame content.
  #[inline]
  pub const fn dlen(&self) -> Option<u32> {
    self.dlen
  }

  fn from_reader<R>(bitflags: FrameV4Flags, reader: &mut R) -> Result<Self>
  where
    R: ReadExt,
  {
    let mut this: Self = Self {
      grid: None,
      encr: None,
      dlen: None,
    };

    // Set to true if the DATA_LENGTH_INDICATOR must be set.
    let mut require_dlen: bool = false;

    if bitflags.contains(FrameV4Flags::GROUPING_IDENTITY) {
      this.grid = Some(reader.read_u8()?);
    }

    if bitflags.contains(FrameV4Flags::COMPRESSION) {
      require_dlen = true;
    }

    // Note: May require `DATA_LENGTH_INDICATOR` but depends on the encr method.
    if bitflags.contains(FrameV4Flags::ENCRYPTION) {
      this.encr = Some(reader.read_u8()?);
    }

    // Note: May include `DATA_LENGTH_INDICATOR` but not mandatory.
    if bitflags.contains(FrameV4Flags::UNSYNCHRONISATION) {
      panic!("TODO: Handle UNSYNCHRONISATION - V4");
    }

    if bitflags.contains(FrameV4Flags::DATA_LENGTH_INDICATOR) {
      // TODO: This is a synchsafe integer.
      this.dlen = Some(reader.read_u32()?);
    } else if require_dlen {
      return Err(Error::new(ErrorKind::InvalidBitFlag));
    }

    Ok(this)
  }
}
