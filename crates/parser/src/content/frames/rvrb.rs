// =============================================================================
// Reverb
// =============================================================================

/// Reverb frame content.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Rvrb {
  reverb_lhs: u16,
  reverb_rhs: u16,
  reverb_bounces_lhs: u8,
  reverb_bounces_rhs: u8,
  reverb_feedback_ltl: u8,
  reverb_feedback_ltr: u8,
  reverb_feedback_rtr: u8,
  reverb_feedback_rtl: u8,
  premix_ltr: u8,
  premix_rtl: u8,
}
