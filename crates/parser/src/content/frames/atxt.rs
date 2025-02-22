use alloc::borrow::Cow;
use bitflags::bitflags;

use crate::decode::Encoding;
use crate::types::Slice;

// =============================================================================
// Audio Text
// =============================================================================

/// Audio text frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Atxt<'a> {
  text_encoding: Encoding,
  mime_type: Cow<'a, str>,
  flags: AtxtFlags,
  text_content: Cow<'a, str>,
  audio_data: Cow<'a, Slice>,
}

// =============================================================================
// Audio Text Flags
// =============================================================================

bitflags! {
  /// Audio text flags.
  #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
  pub struct AtxtFlags: u8 {
    /// Scrambling flag.
    ///
    /// Indicates whether or not scrambling has been applied to the audio data.
    const SCRAMBLE = 0b00000001;
  }
}

decode_bitflags!(AtxtFlags);
copy_into_owned!(AtxtFlags);
