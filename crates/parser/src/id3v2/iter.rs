use crate::error::Result;
use crate::frame::DynFrame;
use crate::id3v2::Header;
use crate::id3v2::Tag;
use crate::types::Slice;

// =============================================================================
// DynFrame Iterator
// =============================================================================

/// An iterator over the frames on an ID3v2 tag.
///
/// This struct is created by the [`frames`][Tag::frames] method on [`tags`][Tag].
#[derive(Clone)]
pub struct FrameIter<'tag> {
  header: &'tag Header,
  buffer: &'tag Slice,
}

impl<'tag> FrameIter<'tag> {
  pub(crate) const fn new(tag: &'tag Tag) -> Self {
    Self {
      header: tag.header(),
      buffer: tag.buffer(),
    }
  }
}

impl<'tag> Iterator for FrameIter<'tag> {
  type Item = Result<DynFrame<'tag>>;

  fn next(&mut self) -> Option<Self::Item> {
    // Exit early if the buffer is empty.
    if self.buffer.is_empty() {
      return None;
    }

    // Read the next frame from the ID3 tag buffer.
    match DynFrame::from_slice(self.header.version(), self.buffer) {
      Ok(None) => {
        // The frame ID was NULL and we don't know how far ahead to skip
        // so we'll just skip to the end of the buffer and stop iterating.
        self.buffer = Slice::empty();

        // Return `None` since this wasn't even a valid frame.
        None
      }
      Ok(Some(frame)) => {
        // The frame was valid so advance the buffer.
        self.buffer = self.buffer.skip(frame.total_size());

        // Return the parsed frame.
        Some(Ok(frame))
      }
      Err(error) => {
        // The frame was invalid and we don't know how far ahead to skip
        // so we'll just skip to the end of the buffer and stop iterating.
        self.buffer = Slice::empty();

        // Return whatever error was encountered.
        Some(Err(error))
      }
    }
  }
}
