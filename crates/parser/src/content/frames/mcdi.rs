use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Music CD Identifier
// =============================================================================

/// Music CD identifier frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Mcdi<'a> {
  #[frame(info = "CD table of contents")]
  data: Cow<'a, Slice>,
}
