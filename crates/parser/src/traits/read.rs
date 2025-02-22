use byteorder::ReadBytesExt;
use byteorder::BE;
use std::io::Read;
use std::io::Result;

use crate::types::Bytes;
use crate::utils;

// =============================================================================
// ReadExt
// =============================================================================

/// Extends [`Read`] with methods for reading number and context-specific data.
pub trait ReadExt: Read {
  /// Reads an `S`-sized array of bytes from the underlying reader.
  fn read_array<const S: usize>(&mut self) -> Result<[u8; S]> {
    let mut data: [u8; S] = [0; S];

    <Self as Read>::read_exact(self, &mut data)?;

    Ok(data)
  }

  /// Reads an owned slice of bytes from the underlying reader.
  fn read_bytes(&mut self, size: usize) -> Result<Bytes> {
    let mut data: Box<[u8]> = vec![0; size].into_boxed_slice();

    <Self as Read>::read_exact(self, &mut data)?;

    Ok(Bytes::new(data))
  }

  /// Reads all bytes until EOF from the underlying reader.
  fn read_all(&mut self, capacity: Option<usize>) -> Result<Bytes> {
    let capacity: usize = capacity.unwrap_or(32);
    let mut data: Vec<u8> = Vec::with_capacity(capacity);

    <Self as Read>::read_to_end(self, &mut data)?;

    Ok(Bytes::new(data.into_boxed_slice()))
  }

  // ===========================================================================
  // Signed Integers
  // ===========================================================================

  /// Reads a signed 8-bit integer from the underlying reader.
  #[inline]
  fn read_i8(&mut self) -> Result<i8> {
    <Self as ReadBytesExt>::read_i8(self)
  }

  // ===========================================================================
  // Unsigned Integers
  // ===========================================================================

  /// Reads an unsigned 8-bit integer from the underlying reader.
  #[inline]
  fn read_u8(&mut self) -> Result<u8> {
    <Self as ReadBytesExt>::read_u8(self)
  }

  /// Reads an unsigned 16-bit integer from the underlying reader.
  #[inline]
  fn read_u16(&mut self) -> Result<u16> {
    <Self as ReadBytesExt>::read_u16::<BE>(self)
  }

  /// Reads an unsigned 24-bit integer from the underlying reader.
  #[inline]
  fn read_u24(&mut self) -> Result<u32> {
    <Self as ReadExt>::read_array(self).map(utils::decode_u24)
  }

  /// Reads an unsigned 32-bit integer from the underlying reader.
  #[inline]
  fn read_u32(&mut self) -> Result<u32> {
    <Self as ReadBytesExt>::read_u32::<BE>(self)
  }

  // ===========================================================================
  // Unsynchronized Integers
  // ===========================================================================

  /// Reads a 28-bit unsynchronized integer from the underlying reader.
  #[inline]
  fn read_u28_unsync(&mut self) -> Result<u32> {
    <Self as ReadExt>::read_array(self).map(utils::decode_u28_unsync)
  }

  /// Reads a 35-bit unsynchronized integer from the underlying reader.
  #[inline]
  fn read_u35_unsync(&mut self) -> Result<u32> {
    <Self as ReadExt>::read_array(self).map(utils::decode_u35_unsync)
  }
}

/// All types that implement `Read` get methods defined by `ReadExt`.
impl<T: Read + ?Sized> ReadExt for T {}
