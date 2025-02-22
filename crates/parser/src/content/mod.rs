//! ID3v2 Frame Content

use crate::error::Result;
use crate::types::Slice;
use crate::types::Version;

// =============================================================================
// Content
// =============================================================================

/// Decoded frame content.
#[derive(Clone, Hash, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Content<'a> {
  /// TODO
  Fixme(&'a ()),
}

impl<'a> Content<'a> {
  pub(crate) fn decode(_version: Version, _name: &str, _slice: &'a Slice) -> Result<Self> {
    panic!("TODO: Content::decode");
  }
}

impl Content<'static> {
  pub(crate) fn decode2(
    _version: Version,
    _name: &str,
    _slice: &Slice,
    _size: u32,
  ) -> Result<Self> {
    panic!("TODO: Content::decode2");
  }
}
