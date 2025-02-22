use core::str::from_utf8;

#[cfg(feature = "zlib")]
use flate2::read::ZlibDecoder;

use crate::error::Result;
use crate::types::Bytes;
use crate::types::Slice;

#[cfg(feature = "zlib")]
use crate::traits::ReadExt;

// =============================================================================
// Unsynchronized
// =============================================================================

const MASK: u8 = 0b11111111;

/// Decode an unsigned 35-bit "unsynchronized" integer from an array of bytes.
pub const fn decode_u35_unsync(bytes: [u8; 5]) -> u32 {
  debug_assert!(bytes[0] & MASK == bytes[0]);
  debug_assert!(bytes[1] & MASK == bytes[1]);
  debug_assert!(bytes[2] & MASK == bytes[2]);
  debug_assert!(bytes[3] & MASK == bytes[3]);
  debug_assert!(bytes[4] & MASK == bytes[4]);

  let mut output: u32 = 0;
  output |= (bytes[0] as u32) << 28;
  output |= (bytes[1] as u32) << 21;
  output |= (bytes[2] as u32) << 14;
  output |= (bytes[3] as u32) << 7;
  output |= (bytes[4] as u32) << 0;
  output
}

/// Decode an unsigned 28-bit "unsynchronized" integer from an array of bytes.
pub const fn decode_u28_unsync(bytes: [u8; 4]) -> u32 {
  debug_assert!(bytes[0] & MASK == bytes[0]);
  debug_assert!(bytes[1] & MASK == bytes[1]);
  debug_assert!(bytes[2] & MASK == bytes[2]);
  debug_assert!(bytes[3] & MASK == bytes[3]);

  let mut output: u32 = 0;
  output |= (bytes[0] as u32) << 21;
  output |= (bytes[1] as u32) << 14;
  output |= (bytes[2] as u32) << 7;
  output |= (bytes[3] as u32) << 0;
  output
}

// =============================================================================
// Misc. Integers
// =============================================================================

/// Decodes an unsigned 24-bit integer from an array of bytes.
pub const fn decode_u24(bytes: [u8; 3]) -> u32 {
  let mut output: u32 = 0;
  output |= (bytes[0] as u32) << 16;
  output |= (bytes[1] as u32) << 8;
  output |= (bytes[2] as u32) << 0;
  output
}

// =============================================================================
// Text Validation
// =============================================================================

/// Returns `true` if the input bytes are a valid frame ID.
///
/// A frame ID is composed of characters A-Z and 0-9.
pub const fn is_frame_id(mut input: &[u8]) -> bool {
  while let [b'A'..=b'Z' | b'0'..=b'9', tail @ ..] = input {
    input = tail;
  }

  input.is_empty()
}

/// Returns `true` if the input bytes are a valid ISO-8859-1 string.
///
/// Note: All characters must be in the range of `0x20-0xFF`.
pub const fn is_latin1(mut input: &[u8]) -> bool {
  while let [0x20..=0xFF, tail @ ..] = input {
    input = tail;
  }

  input.is_empty()
}

/// Returns `true` if the input bytes are valid ASCII digits.
pub const fn is_ascii_digit(mut input: &[u8]) -> bool {
  while let [b'0'..=b'9', tail @ ..] = input {
    input = tail;
  }

  input.is_empty()
}

/// Returns `true` if the input bytes are valid UTF-8.
pub const fn is_utf8(input: &[u8]) -> bool {
  from_utf8(input).is_ok()
}

/// Returns `true` if the input bytes are all NULL (0x00).
pub const fn is_null(mut input: &[u8]) -> bool {
  while let [0x00, tail @ ..] = input {
    input = tail;
  }

  input.is_empty()
}

// =============================================================================
// Compression
// =============================================================================

#[cfg(feature = "zlib")]
pub fn decompress(input: &Slice, size: Option<usize>) -> Result<Bytes> {
  ZlibDecoder::new(input.cursor())
    .read_all(size)
    .map_err(Into::into)
}

#[cfg(not(feature = "zlib"))]
pub fn decompress(_input: &Slice, _size: Option<usize>) -> Result<Bytes> {
  panic!("Enable `zlib` feature to use ZLIB decompression.");
}
