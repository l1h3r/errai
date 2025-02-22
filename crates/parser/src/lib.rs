//! Errai - ID3 Metadata Reader

#![deny(missing_docs)]

extern crate alloc;

#[macro_use]
extern crate derive;

#[macro_use]
mod macros;

mod decode;
mod traits;
mod utils;

pub mod content;
pub mod error;
pub mod frame;
pub mod id3v2;
pub mod types;
pub mod unsync;
