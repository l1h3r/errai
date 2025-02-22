use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Audio Encryption
// =============================================================================

/// Audio encryption frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Aenc<'a> {
  owner_identifier: Cow<'a, str>,
  preview_start: u16,
  preview_length: u16,
  encryption_info: Cow<'a, Slice>,
}
