use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Relative Volume Adjustment (2)
// =============================================================================

/// Relative volume adjustment (2) frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Rva2<'a> {
  fixme: Cow<'a, Slice>,
}
