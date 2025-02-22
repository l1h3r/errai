use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Relative Volume Adjustment
// =============================================================================

/// Relative volume adjustment frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Rvad<'a> {
  fixme: Cow<'a, Slice>,
}
