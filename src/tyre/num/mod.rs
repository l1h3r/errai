//! Atomic numeric types with safety invariants.
//!
//! - [`AtomicNzU64`]: Atomic u64 never zero

mod atomic_nz;

pub use self::atomic_nz::AtomicNzU64;
