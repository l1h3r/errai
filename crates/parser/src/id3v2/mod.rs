//! ID3v2 Support

mod extend;
mod header;
mod iter;
mod tag;

pub use self::extend::ExtHeader;
pub use self::extend::ExtHeaderFlags;
pub use self::extend::ExtHeaderFlagsV3;
pub use self::extend::ExtHeaderFlagsV4;
pub use self::extend::ImageEncRestriction;
pub use self::extend::ImageLenRestriction;
pub use self::extend::Restrictions;
pub use self::extend::TagSizeRestriction;
pub use self::extend::TextEncRestriction;
pub use self::extend::TextLenRestriction;
pub use self::header::Header;
pub use self::header::HeaderFlags;
pub use self::iter::FrameIter;
pub use self::tag::Tag;
