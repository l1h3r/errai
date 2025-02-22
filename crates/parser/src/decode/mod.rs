//! ID3v2 Frame Content Decoding

mod decoder;
mod encoding;

pub use self::decoder::Decode;
pub use self::decoder::Decoder;
pub use self::encoding::Encoding;
