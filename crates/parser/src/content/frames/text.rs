use alloc::borrow::Cow;

use crate::decode::Encoding;

// =============================================================================
// Text Information
// =============================================================================

/// Text information frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Text<'a> {
  text_encoding: Encoding,
  text_content: Cow<'a, str>,
}
