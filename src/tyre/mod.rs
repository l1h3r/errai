//! Low-level concurrent programming primitives.
//!
//! # Modules
//!
//! - [`num`]: Atomic types with invariants
//! - [`ptr`]: Tagged pointer operations

#[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
compile_error!("tyre requires a 32-bit or 64-bit platform");

pub mod num;
pub mod ptr;
