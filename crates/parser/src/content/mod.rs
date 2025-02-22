//! ID3v2 Frame Content

use crate::error::Result;
use crate::traits::IntoOwned;
use crate::types::Bytes;
use crate::types::Slice;
use crate::types::Version;
use crate::utils;

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
  pub(crate) fn decode2(version: Version, name: &str, slice: &Slice, size: u32) -> Result<Self> {
    let bytes: Bytes = utils::decompress(slice, Some(size as usize))?;
    let slice: &Slice = bytes.as_slice();

    let content: Content<'_> = Content::decode(version, name, slice)?;

    Ok(content.into_owned())
  }
}

impl IntoOwned for Content<'_> {
  type Owned = Content<'static>;

  fn into_owned(self) -> Self::Owned {
    panic!("Content::into_owned");
  }
}
