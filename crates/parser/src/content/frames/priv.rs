use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Private
// =============================================================================

/// Private frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Priv<'a> {
  owner_identifier: Cow<'a, str>,
  private_data: Cow<'a, Slice>,
}
