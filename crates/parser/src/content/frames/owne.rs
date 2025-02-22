use alloc::borrow::Cow;

use crate::decode::Date;
use crate::decode::Encoding;

// =============================================================================
// Ownership Frame
// =============================================================================

/// Ownership frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Owne<'a> {
  text_encoding: Encoding,
  #[frame(read = "@latin1")]
  price_paid: Cow<'a, str>,
  purchase_date: Date,
  seller: Cow<'a, str>,
}
