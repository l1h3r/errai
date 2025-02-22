use bitflags::bitflags;

use crate::error::Error;
use crate::error::Result;
use crate::error::TagField;
use crate::id3v2::ExtHeader;
use crate::traits::ReadExt;
use crate::types::Version;

// =============================================================================
// Header
// =============================================================================

/// A parsed ID3v2 header.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Header {
  version: Version,
  bitflags: HeaderFlags,
  data_len: u32,
  exheader: Option<ExtHeader>,
}

impl Header {
  /// ID3 tag identifier.
  pub const IDENTIFIER: [u8; 3] = *b"ID3";

  /// Get the ID3 tag version.
  #[inline]
  pub const fn version(&self) -> Version {
    self.version
  }

  /// Get the ID3 tag bitflags.
  #[inline]
  pub const fn bitflags(&self) -> HeaderFlags {
    self.bitflags
  }

  /// Get the ID3 tag size (in bytes).
  ///
  /// Note: This is offset the the extended header size (if included).
  #[inline]
  pub const fn data_len(&self) -> u32 {
    match self.exheader() {
      Some(header) => self.data_len - header.ext_size(),
      None => self.data_len,
    }
  }

  /// Get a shared reference to the extended header (if included).
  #[inline]
  pub const fn exheader(&self) -> Option<&ExtHeader> {
    self.exheader.as_ref()
  }

  /// Returns `true` if the `UNSYNCHRONISATION` flag is set (and applicable).
  #[inline]
  pub const fn flag_unsynchronisation(&self) -> bool {
    self.bitflags.contains(HeaderFlags::UNSYNCHRONISATION)
  }

  /// Returns `true` if the `COMPRESSION` flag is set (and applicable).
  #[inline]
  pub const fn flag_compression(&self) -> bool {
    let Version::ID3v22 = self.version else {
      return false;
    };

    self.bitflags.contains(HeaderFlags::COMPRESSION)
  }

  /// Returns `true` if the `EXTENDED_HEADER` flag is set (and applicable).
  #[inline]
  pub const fn flag_extended_header(&self) -> bool {
    let (Version::ID3v23 | Version::ID3v24) = self.version else {
      return false;
    };

    self.bitflags.contains(HeaderFlags::EXTENDED_HEADER)
  }

  /// Returns `true` if the `EXPERIMENTAL` flag is set (and applicable).
  #[inline]
  pub const fn flag_experimental(&self) -> bool {
    let (Version::ID3v23 | Version::ID3v24) = self.version else {
      return false;
    };

    self.bitflags.contains(HeaderFlags::EXPERIMENTAL)
  }

  /// Returns `true` if the `FOOTER_PRESENT` flag is set (and applicable).
  #[inline]
  pub const fn flag_footer(&self) -> bool {
    let Version::ID3v24 = self.version else {
      return false;
    };

    self.bitflags.contains(HeaderFlags::FOOTER_PRESENT)
  }

  /// Parse an ID3v2 tag header from the given `reader`.
  pub fn from_reader<R>(mut reader: R) -> Result<Self>
  where
    R: ReadExt,
  {
    // Always "ID3" to indicate that this is an ID3 tag.
    if reader.read_array()? != Header::IDENTIFIER {
      return Err(Error::tag(TagField::Identifier));
    }

    // 2 bytes - [major, revision].
    let version: Version = match reader.read_array()? {
      [0x02, _] => Version::ID3v22,
      [0x03, _] => Version::ID3v23,
      [0x04, _] => Version::ID3v24,
      [_, _] => return Err(Error::tag(TagField::Version)),
    };

    // 1 byte - valid flags are heavily dependant on version.
    let bitflags: HeaderFlags = HeaderFlags::from_reader(&mut reader)?;

    // 28-bit "unsynchronized" integer.
    let data_len: u32 = reader.read_u28_unsync()?;

    let mut this: Self = Self {
      version,
      bitflags,
      data_len,
      exheader: None,
    };

    if this.flag_extended_header() {
      if this.flag_unsynchronisation() {
        panic!("TODO: Handle UNSYNCHRONISATION");
      }

      let exheader: ExtHeader = match this.version() {
        Version::ID3v11 => unreachable!(),
        Version::ID3v12 => unreachable!(),
        Version::ID3v22 => return Err(Error::tag(TagField::Version)),
        Version::ID3v23 => ExtHeader::from_reader_v3(&mut reader)?,
        Version::ID3v24 => ExtHeader::from_reader_v4(&mut reader)?,
      };

      this.exheader = Some(exheader);
    }

    Ok(this)
  }
}

// =============================================================================
// Header Flags
// =============================================================================

bitflags! {
  /// ID3v2 header flags.
  #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
  pub struct HeaderFlags: u8 {
    /// Unsynchronisation.
    ///
    /// Indicates whether or not unsynchronisation is applied to all frames.
    ///
    /// Note: Applicable to all versions.
    const UNSYNCHRONISATION = 0b10000000;
    /// Indicates whether or not compression is used.
    ///
    /// Note: Only applicable to `ID3v2.2`.
    const COMPRESSION = 0b01000000;
    /// Extended header.
    ///
    /// Indicates whether or not the header is followed by an extended header.
    ///
    /// Note: Only applicable to `ID3v2.3`/`ID3v2.4`.
    const EXTENDED_HEADER = 0b01000000;
    /// Experimental indicator.
    ///
    /// Always set when the tag is in an experimental stage.
    ///
    /// Note: Only applicable to `ID3v2.3`/`ID3v2.4`.
    const EXPERIMENTAL = 0b00100000;
    /// Footer present.
    ///
    /// Indicates that a footer is present at the very end of the tag.
    ///
    /// Note: Only applicable to `ID3v2.4`.
    const FOOTER_PRESENT = 0b00010000;
  }
}

impl HeaderFlags {
  fn from_reader<R>(reader: &mut R) -> Result<Self>
  where
    R: ReadExt,
  {
    reader
      .read_u8()
      .map_err(Into::into)
      .map(Self::from_bits_retain)
  }
}
