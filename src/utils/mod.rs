//! Utility types and functions used throughout the runtime.
//!
//! # Contents
//!
//! - [`CatchUnwind`]: Future wrapper for catching panics

mod futures;

pub(crate) use self::futures::CatchUnwind;
