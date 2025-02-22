use alloc::borrow::Cow;

use crate::decode::Encoding;
use crate::decode::Language;

// =============================================================================
// Unsychronised Lyrics
// =============================================================================

/// Unsychronised lyrics frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Uslt<'a> {
  text_encoding: Encoding,
  language: Language,
  content_descriptor: Cow<'a, str>,
  lyrics: Cow<'a, str>,
}
