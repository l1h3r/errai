use alloc::borrow::Cow;
use std::io::Cursor;

use crate::decode::Encoding;
use crate::error::Error;
use crate::error::ErrorKind;
use crate::error::Result;
use crate::frame::DynFrame;
use crate::traits::ReadExt;
use crate::types::FrameId;
use crate::types::Slice;
use crate::types::Version;

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
    Self::with_format(input, Encoding::Latin1)
  }

  /// Create a new content `Decoder` with the given text `format`.
  #[inline]
  pub fn with_format(input: &'a Slice, format: Encoding) -> Self {
    Self {
      cursor: Cursor::new(input),
      format,
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

  /// Decode an embedded frame.
  pub fn decode_frame(&mut self, version: Version) -> Result<Option<DynFrame<'a>>> {
    let slice: &Slice = self.cursor.get_ref();
    let index: u64 = self.cursor.position();

    match DynFrame::from_slice(version, slice) {
      Ok(Some(frame)) => {
        self.cursor.set_position(index + frame.total_size() as u64);
        Ok(Some(frame))
      }
      Ok(None) => {
        Ok(None)
      }
      Err(error) => {
        Err(error)
      }
    }
  }

  /// Returns `true` if the decoder is empty.
  pub fn is_empty(&self) -> bool {
    // TODO: Use cursor.is_empty() when stable
    //
    // https://github.com/rust-lang/rust/issues/86369
    self.cursor.position() >= self.cursor.get_ref().len() as u64
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

  pub(crate) fn step<F>(&mut self, offset: u64, f: F) -> &'a Slice
  where
    F: FnOnce(&'a Slice) -> &'a Slice,
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

impl<'a> Decode<'a> for Cow<'a, Slice> {
  #[inline]
  fn decode(decoder: &mut Decoder<'a>) -> Result<Self> {
    Ok(Cow::Borrowed(decoder.remaining()))
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

macro_rules! impl_nonzero {
  ($nonzero:ident) => {
    impl Decode<'_> for ::core::num::$nonzero {
      #[inline]
      fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
        decoder
          .decode()
          .map(::core::num::$nonzero::new)
          .transpose()
          .ok_or_else(|| Error::new(ErrorKind::InvalidFrameData))?
      }
    }
  };
  ($($nonzero:ident),+) => {
    $(
      impl_nonzero!($nonzero);
    )+
  };
}

impl_nonzero!(NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64);
impl_nonzero!(NonZeroI8, NonZeroI16, NonZeroI32, NonZeroI64);

// =============================================================================
// Implementations for Crate Types
// =============================================================================

impl<const S: usize> Decode<'_> for FrameId<S> {
  #[inline]
  fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
    decoder.decode::<[u8; S]>().and_then(Self::try_from)
  }
}
