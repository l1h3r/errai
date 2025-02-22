use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Equalization
// =============================================================================

/// Equalization frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Equa<'a> {
  fixme: Cow<'a, Slice>,
}
