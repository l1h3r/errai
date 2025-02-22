use alloc::borrow::Cow;
use bitflags::bitflags;
use core::num::NonZeroU8;

use crate::decode::Decode;
use crate::decode::Decoder;
use crate::error::Result;
use crate::frame::DynFrame;
use crate::types::Slice;

// =============================================================================
// Table of Contents
// =============================================================================

/// Table of contents frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Ctoc<'a> {
  element_identifier: Cow<'a, str>,
  bitflags: CtocFlags,
  entry_count: NonZeroU8,
  binary_data: Cow<'a, Slice>,
}

impl Ctoc<'_> {
  /// Get an iterator over the elements of the frame.
  pub fn elements(&self) -> CtocIter<'_> {
    CtocIter::new(self.entry_count.get(), self.binary_data())
  }
}

// =============================================================================
// Table of Contents Flags
// =============================================================================

bitflags! {
  /// Table of contents flags.
  #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
  pub struct CtocFlags: u8 {
    /// Top-level.
    ///
    /// Indicates that this frame is the root of the Table of Contents tree
    /// and is not a child of any other "CTOC" frame.
    const TOP_LEVEL = 0b00000010;
    /// Ordered.
    ///
    /// Indicates whether the entries in the Child Element ID list are ordered.
    const ORDERED = 0b00000000;
  }
}

impl Decode<'_> for CtocFlags {
  #[inline]
  fn decode(decoder: &mut Decoder<'_>) -> Result<Self> {
    decoder.decode().map(Self::from_bits_retain)
  }
}

copy_into_owned!(CtocFlags);

// =============================================================================
// Child Element
// =============================================================================

/// An element or embedded frame within a [`CTOC`][Ctoc] frame.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum CtocItem<'a> {
  /// Child element identifier.
  Entry(Cow<'a, str>),
  /// Embedded frame.
  Frame(DynFrame<'a>),
}

// =============================================================================
// Ctoc Iterator
// =============================================================================

/// An iterator over the elements of a [`CTOC`][Ctoc] frame.
#[derive(Clone, Debug)]
pub struct CtocIter<'a> {
  count: u8,
  index: u8,
  inner: Decoder<'a>,
}

impl<'a> CtocIter<'a> {
  fn new(count: u8, input: &'a Slice) -> Self {
    Self {
      count,
      index: 0,
      inner: Decoder::new(input),
    }
  }
}

impl<'a> Iterator for CtocIter<'a> {
  type Item = Result<CtocItem<'a>>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.inner.is_empty() {
      return None;
    }

    if self.index < self.count {
      self.index += 1;
      Some(self.inner.decode().map(CtocItem::Entry))
    } else {
      panic!("TODO: Parse Embedded Frame");
    }
  }
}
