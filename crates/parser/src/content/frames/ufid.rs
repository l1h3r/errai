use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Unique File Identifier
// =============================================================================

/// Unique file identifier frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Ufid<'a> {
  // TODO: Validate that this is non-empty
  owner_identifier: Cow<'a, str>,
  // TODO: Validate that this is up to 64 bytes
  identifier: Cow<'a, Slice>,
}
