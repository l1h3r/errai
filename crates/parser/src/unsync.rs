//! ID3 Unsynchronisation Scheme

use std::io::ErrorKind;
use std::io::Read;
use std::io::Result;

use crate::traits::ReadExt;

// =============================================================================
// Unsync Reader
// =============================================================================

/// Implementation of [`Read`] for ID3 unsynchronisation scheme.
#[derive(Debug)]
pub struct Unsync<R> {
  reader: R,     // The actual reader implementation.
  bcache: u8,    // The last byte we read.
  bstale: bool,  // Whether we wrote the last byte to the output buffer.
  cursor: usize, // Current position in the output buffer.
}

impl<R> Unsync<R> {
  /// Create a new `Unsync` reader.
  pub const fn new(reader: R) -> Self {
    Self {
      reader,
      bcache: 0,
      bstale: false,
      cursor: 0,
    }
  }
}

impl<R> Read for Unsync<R>
where
  R: Read,
{
  fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
    let length: usize = buffer.len();

    // Bail early if the buffer is empty.
    if length == 0 {
      return Ok(0);
    }

    // Reset the cursor position.
    self.cursor = 0;

    loop {
      // Read the next byte or bail on EOF.
      let byte: u8 = match self.reader.read_u8() {
        Ok(byte) => byte,
        Err(error) if error.kind() == ErrorKind::UnexpectedEof => break,
        Err(error) => return Err(error),
      };

      // Only write this byte if we are not currently on a [0xFF, 0x00] pair.
      if !(self.bcache == 0xFF && byte == 0x00) {
        // Write this byte to the output buffer.
        buffer[self.cursor] = byte;

        // Clear the "stale" flag and increment the cursor position.
        self.bstale = false;
        self.cursor += 1;

        // Break the loop if we've reached the end of the buffer.
        if self.cursor == length {
          break;
        }
      } else {
        // Set the "stale" flag since we didn't write this byte.
        self.bstale = true;
      }

      // Store the current byte for next comparison
      self.bcache = byte;
    }

    // Add trailing byte if applicable.
    if self.bstale {
      buffer[self.cursor] = self.bcache;
    }

    // Return the cursor position +1 for trailing byte.
    Ok(self.cursor + usize::from(self.bstale))
  }
}
