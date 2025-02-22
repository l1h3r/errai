use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Unknown Frame Content
// =============================================================================

/// Unknown frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Unkn<'a> {
  binary_data: Cow<'a, Slice>,
}
