//! Errai - ID3 Metadata Reader

#![deny(missing_docs)]

extern crate alloc;

pub mod error;
pub mod frame;
pub mod id3v2;
mod traits;
pub mod types;
pub mod unsync;
mod utils;
