//! Errai - ID3 Metadata Reader

#![deny(missing_docs)]

extern crate alloc;

mod decode;
mod traits;
mod utils;

pub mod content;
pub mod error;
pub mod frame;
pub mod id3v2;
pub mod types;
pub mod unsync;
