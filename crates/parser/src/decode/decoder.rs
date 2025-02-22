use alloc::borrow::Cow;
use std::io::Cursor;

use crate::decode::Encoding;
use crate::error::Result;
use crate::traits::ReadExt;
use crate::types::Slice;

// =============================================================================
// Content Decoder
// =============================================================================

/// Frame content decoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decoder<'a> {
  cursor: Cursor<&'a Slice>,
  format: Encoding,
}

impl<'a> Decoder<'a> {
  /// Create a new content `Decoder`.
  #[inline]
  pub fn new(input: &'a Slice) -> Self {
    Self {
      cursor: Cursor::new(input),
      format: Encoding::Latin1,
    }
  }

  /// Decode a `T` value in ID3v2.3 form.
  ///
  /// To decode an ID3v2.2 structure use [`decode_v2`][Self::decode_v2].
  #[inline]
  pub fn decode<T>(&mut self) -> Result<T>
  where
    T: Decode<'a>,
  {
    T::decode(self)
  }

  /// Decode a `T` value in ID3v2.2 form.
  ///
  /// To decode an ID3v2.3 structure use [`decode`][Self::decode].
  #[inline]
  pub fn decode_v2<T>(&mut self) -> Result<T>
  where
    T: Decode<'a>,
  {
    T::decode_v2(self)
  }

  /// Decode a string in `ISO-8859-1` form.
  #[inline]
  pub fn decode_latin1(&mut self) -> Result<Cow<'a, str>> {
    Encoding::decode(Encoding::Latin1, self)
  }

  /// Get a slice of the remaining bytes in the decoder.
  #[inline]
  pub fn remaining(&mut self) -> &'a Slice {
    self.step(0, |slice| slice)
  }

  /// Get a slice of the remaining bytes up to the first NUL byte.
  #[inline]
  pub fn until_nul(&mut self) -> &'a Slice {
    self.step(1, Slice::until_nul)
  }

  /// Get a slice of the remaining bytes up to the first NUL byte pair.
  #[inline]
  pub fn until_nul2(&mut self) -> &'a Slice {
    self.step(2, Slice::until_nul2)
  }

  pub(crate) fn set_format(&mut self, encoding: Encoding) {
    self.format = encoding;
  }

  fn step<F>(&mut self, offset: u64, f: F) -> &'a Slice
  where
    F: FnOnce(&Slice) -> &Slice,
  {
    let slice: &Slice = self.cursor.get_ref();
    let index: u64 = self.cursor.position().min(slice.len() as u64);
    let bytes: &Slice = f(&slice[(index as usize)..]);
    let ahead: u64 = index + bytes.len() as u64;

    self.cursor.set_position(ahead + offset);

    bytes
  }
}

// =============================================================================
// Decode
// =============================================================================

/// Content decoding behaviour.
pub trait Decode<'a>: Sized {
  /// Decode `Self` in ID3v2.3 form.
  fn decode(decoder: &mut Decoder<'a>) -> Result<Self>;

  /// Decode `Self` in ID3v2.2 form.
  #[inline]
  fn decode_v2(decoder: &mut Decoder<'a>) -> Result<Self> {
    Decode::decode(decoder)
  }
}

// =============================================================================
// Implementations for Rust Types
// =============================================================================

impl<const S: usize> Decode<'_> for [u8; S] {
  #[inline]
  fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
    decoder.cursor.read_array().map_err(Into::into)
  }
}

impl<'a> Decode<'a> for Cow<'a, str> {
  #[inline]
  fn decode(decoder: &mut Decoder<'a>) -> Result<Self> {
    decoder.format.decode(decoder)
  }
}

macro_rules! impl_integer {
  ($integer:ty) => {
    impl Decode<'_> for $integer {
      #[inline]
      fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
        decoder.decode().map(<$integer>::from_be_bytes)
      }
    }
  };
  ($($integer:ty),+) => {
    $(
      impl_integer!($integer);
    )+
  };
}

impl_integer!(u8, u16, u32, u64);
impl_integer!(i8, i16, i32, i64);
