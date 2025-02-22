use alloc::borrow::Cow;

use crate::decode::Decoder;
use crate::error::Result;
use crate::types::FrameId;
use crate::types::Slice;

// =============================================================================
// Linked Information
// =============================================================================

/// Linked information frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Link<'a> {
  #[frame(copy)]
  frame_identifier: FrameId<3>,
  url: Cow<'a, str>,
  additional_data: Cow<'a, Slice>,
}

impl Link<'_> {
  /// Get an iterator over the text entries of the frame.
  #[inline]
  pub fn text(&self) -> LinkIter<'_> {
    LinkIter::new(self.additional_data())
  }
}

// =============================================================================
// Link Iterator
// =============================================================================

/// An iterator over the text entries of a [`LINK`][Link] frame.
#[derive(Clone, Debug)]
pub struct LinkIter<'a> {
  inner: Decoder<'a>,
}

impl<'a> LinkIter<'a> {
  fn new(input: &'a Slice) -> Self {
    Self {
      inner: Decoder::new(input),
    }
  }
}

impl<'a> Iterator for LinkIter<'a> {
  type Item = Result<Cow<'a, str>>;

  fn next(&mut self) -> Option<Self::Item> {
    if !self.inner.is_empty() {
      Some(self.inner.decode())
    } else {
      None
    }
  }
}
