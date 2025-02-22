use bitflags::bitflags;

// =============================================================================
// Recommended Buffer Size
// =============================================================================

/// Recommended buffer size frame content.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Rbuf {
  buffer_size: u32,
  bitflags: RbufFlags,
  tag_offset: u32,
}

// =============================================================================
// Recommended Buffer Size Flags
// =============================================================================

bitflags! {
  /// Recommended buffer size flags.
  #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
  pub struct RbufFlags: u8 {
    /// Embedded Info flag.
    ///
    /// Indicates that an ID3 tag with the maximum size described
    /// in 'Buffer size' may occur in the audiostream.
    const EMBEDDED_INFO = 0b00000001;
  }
}

decode_bitflags!(RbufFlags);
copy_into_owned!(RbufFlags);
