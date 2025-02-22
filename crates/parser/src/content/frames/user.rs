use alloc::borrow::Cow;

use crate::decode::Encoding;
use crate::decode::Language;

// =============================================================================
// Terms of Use
// =============================================================================

/// Terms of use frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct User<'a> {
  text_encoding: Encoding,
  language: Language,
  text_content: Cow<'a, str>,
}
