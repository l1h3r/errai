use alloc::borrow::Cow;

// =============================================================================
// Popularimeter
// =============================================================================

/// Popularimeter frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Popm<'a> {
  user_email: Cow<'a, str>,
  rating: u8,
  // TODO: Confirm 8-byte value is valid.
  #[frame(read = "@u64")]
  counter: u64,
}
