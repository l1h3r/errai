//! Raw pointer types.

mod atomic;
mod tagged;

pub use self::atomic::AtomicTaggedPtr;
pub use self::tagged::TaggedPtr;
