//! Common Utility Types

mod bytes;
mod frame;
mod slice;
mod version;

pub use self::bytes::Bytes;
pub use self::frame::FrameId;
pub use self::slice::Slice;
pub use self::version::Version;
