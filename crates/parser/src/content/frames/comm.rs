use alloc::borrow::Cow;

use crate::decode::Encoding;
use crate::decode::Language;

// =============================================================================
// Comments
// =============================================================================

/// Comments frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Comm<'a> {
  text_encoding: Encoding,
  language: Language,
  text_summary: Cow<'a, str>,
  text_details: Cow<'a, str>,
}
