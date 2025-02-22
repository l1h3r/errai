use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Synchronized Tempo Codes
// =============================================================================

/// Synchronized tempo codes frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Sytc<'a> {
  fixme: Cow<'a, Slice>,
}
