use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Group Identification Registration
// =============================================================================

/// Group identification registration frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Grid<'a> {
  owner_identifier: Cow<'a, str>,
  group_symbol: u8,
  group_data: Cow<'a, Slice>,
}
