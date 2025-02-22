// =============================================================================
// Play Counter
// =============================================================================

/// Play counter frame content.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Pcnt {
  // TODO: Confirm 8-byte value is valid.
  #[frame(read = "@u64")]
  counter: u64,
}
