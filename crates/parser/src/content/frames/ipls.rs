use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Involved People List
// =============================================================================

/// Involved people list frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Ipls<'a> {
  fixme: Cow<'a, Slice>,
}
