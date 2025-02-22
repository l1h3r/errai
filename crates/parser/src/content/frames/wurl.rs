use alloc::borrow::Cow;

// =============================================================================
// Web URL Link
// =============================================================================

/// Web URL link frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Wurl<'a> {
  url: Cow<'a, str>,
}
