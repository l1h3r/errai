use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// MPEG Location Lookup Table
// =============================================================================

/// MPEG location lookup table frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Mllt<'a> {
  fixme: Cow<'a, Slice>,
}
