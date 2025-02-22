use core::num::NonZeroU32;
use std::io::Cursor;

use crate::content::Content;
use crate::error::Result;
use crate::traits::ReadExt;
use crate::types::FrameId;
use crate::types::Slice;
use crate::types::Version;

// =============================================================================
// Frame - ID3v2.2
// =============================================================================

/// A parsed ID3v2.2 frame.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameV2<'a> {
  identifier: FrameId<3>,
  descriptor: NonZeroU32,
  frame_data: &'a Slice,
}

impl<'a> FrameV2<'a> {
  /// The size of the frame header (in bytes).
  pub const SIZE: usize = 6;

  /// The version of the frame.
  pub const VERSION: Version = Version::ID3v22;

  /// Get the frame identifier.
  #[inline]
  pub const fn identifier(&self) -> FrameId<3> {
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

  /// Decode the contents of the frame.
  #[inline]
  pub fn decode(&self) -> Result<Content<'a>> {
    Content::decode(Self::VERSION, self.identifier_str(), self.frame_data())
  }

  /// Parse an ID3v2.2 frame from the given `slice`.
  pub fn from_slice(slice: &'a Slice) -> Result<Option<Self>> {
    let mut reader: Cursor<&Slice> = slice.cursor();

    let identifier: FrameId<3> = reader.read_array()?.try_into()?;
    let descriptor: NonZeroU32 = reader.read_u24()?.try_into()?;
    let frame_data: &Slice = reader.get_ref().view(Self::SIZE, descriptor.get() as usize);

    Ok(Some(Self {
      identifier,
      descriptor,
      frame_data,
    }))
  }
}
