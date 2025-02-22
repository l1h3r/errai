use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Position Synchronisation Frame
// =============================================================================

/// Position synchronisation frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Poss<'a> {
  fixme: Cow<'a, Slice>,
}
