use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use crate::error::Result;
use crate::frame::DynFrame;
use crate::id3v2::FrameIter;
use crate::id3v2::Header;
use crate::traits::ReadExt;
use crate::types::Bytes;
use crate::types::Slice;
use crate::unsync::Unsync;

// =============================================================================
// Tag
// =============================================================================

/// A parsed ID3v2 tag.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tag {
  header: Header,
  buffer: Bytes,
}

impl Tag {
  /// Get a shared reference to the tag header.
  #[inline]
  pub const fn header(&self) -> &Header {
    &self.header
  }

  /// Get a shared reference to the tag content.
  #[inline]
  pub const fn buffer(&self) -> &Slice {
    self.buffer.as_slice()
  }

  /// Get an iterator over the frames of the tag.
  #[inline]
  pub const fn frames(&self) -> FrameIter<'_> {
    FrameIter::new(self)
  }

  /// Parse an ID3v2 tag from the file at the given `path`.
  pub fn from_path<P>(path: &P) -> Result<Self>
  where
    P: AsRef<Path> + ?Sized,
  {
    let file: File = File::open(path)?;
    let read: BufReader<File> = BufReader::new(file);

    Self::from_reader(read)
  }

  /// Parse an ID3v2 tag from the given `reader`.
  pub fn from_reader<R>(mut reader: R) -> Result<Self>
  where
    R: ReadExt,
  {
    let header: Header = Header::from_reader(&mut reader)?;
    let length: usize = header.data_len() as usize;

    // Read the entire set of frames, which is sized according to the header.
    let buffer: Bytes = if header.flag_unsynchronisation() {
      Unsync::new(reader).read_bytes(length)?
    } else {
      reader.read_bytes(length)?
    };

    Ok(Self { header, buffer })
  }
}

impl<'tag> IntoIterator for &'tag Tag {
  type Item = Result<DynFrame<'tag>>;
  type IntoIter = FrameIter<'tag>;

  #[inline]
  fn into_iter(self) -> Self::IntoIter {
    self.frames()
  }
}

impl<'tag> IntoIterator for &'tag mut Tag {
  type Item = Result<DynFrame<'tag>>;
  type IntoIter = FrameIter<'tag>;

  #[inline]
  fn into_iter(self) -> Self::IntoIter {
    self.frames()
  }
}
