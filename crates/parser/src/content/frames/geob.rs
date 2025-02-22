use alloc::borrow::Cow;

use crate::decode::Encoding;
use crate::types::Slice;

// =============================================================================
// General Encapsulated Object
// =============================================================================

/// General encapsulated object frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Geob<'a> {
  text_encoding: Encoding,
  #[frame(read = "@latin1")]
  mime_type: Cow<'a, str>,
  filename: Cow<'a, str>,
  content_description: Cow<'a, str>,
  encapsulated_object: Cow<'a, Slice>,
}
