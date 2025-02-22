// =============================================================================
// ID3 Version
// =============================================================================

/// The version of an ID3 tag.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Version {
  /// ID3v1.1
  ID3v11,
  /// ID3v1.2
  ID3v12,
  /// ID3v2.2
  ID3v22,
  /// ID3v2.3
  ID3v23,
  /// ID3v2.4
  ID3v24,
}
