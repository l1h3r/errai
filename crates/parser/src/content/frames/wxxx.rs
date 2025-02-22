use alloc::borrow::Cow;

use crate::decode::Encoding;

// =============================================================================
// User-defined URL Link Frame
// =============================================================================

/// User-defined URL link frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Wxxx<'a> {
  text_encoding: Encoding,
  description: Cow<'a, str>,
  #[frame(read = "@latin1")]
  url: Cow<'a, str>,
}
