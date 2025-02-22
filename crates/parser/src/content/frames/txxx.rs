use alloc::borrow::Cow;

use crate::decode::Encoding;

// =============================================================================
// User-defined Text Information
// =============================================================================

/// User-defined text information frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Txxx<'a> {
  text_encoding: Encoding,
  text_summary: Cow<'a, str>,
  text_details: Cow<'a, str>,
}
