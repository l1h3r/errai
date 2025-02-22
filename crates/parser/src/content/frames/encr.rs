use alloc::borrow::Cow;

use crate::types::Slice;

// =============================================================================
// Encryption Method Registration
// =============================================================================

/// Encryption method registration frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Encr<'a> {
  owner_identifier: Cow<'a, str>,
  method_symbol: u8,
  encryption_data: Cow<'a, Slice>,
}
