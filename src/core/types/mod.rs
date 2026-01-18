//! Fundamental runtime types including PIDs, references, terms, destinations, and exit reasons.
//!
//! This module provides the core type system for the Errai runtime. These
//! types enable process identification, message passing, monitoring, and
//! fault handling.
//!
//! # Type Categories
//!
//! ## Process Identification
//!
//! - [`InternalPid`]: Local process identifier
//! - [`ExternalPid`]: Distributed process identifier
//! - [`ProcessId`]: Trait for generic PID handling
//!
//! ## Message Destinations
//!
//! - [`InternalDest`]: Local destination (PID or name)
//! - [`ExternalDest`]: Distributed destination (local or remote)
//!
//! ## References
//!
//! - [`InternalRef`]: Base reference type
//! - [`ExternalRef`]: Distributed reference
//! - [`MonitorRef`]: Monitor identifier
//! - [`AliasRef`]: Process alias identifier
//! - [`TimerRef`]: Timer identifier
//!
//! ## Values and Exit Reasons
//!
//! - [`Atom`]: Interned string identifier
//! - [`Term`]: Type-erased runtime value
//! - [`Item`]: Trait for values stored in [`Term`]
//! - [`Exit`]: Process termination reason

mod atom;
mod dest;
mod exit;
mod item;
mod pids;
mod refs;
mod term;

pub use self::atom::Atom;
pub use self::dest::ExternalDest;
pub use self::dest::InternalDest;
pub use self::exit::Exit;
pub use self::item::Item;
pub use self::pids::ExternalPid;
pub use self::pids::InternalPid;
pub use self::pids::ProcessId;
pub use self::refs::AliasRef;
pub use self::refs::ExternalRef;
pub use self::refs::InternalRef;
pub use self::refs::MonitorRef;
pub use self::refs::TimerRef;
pub use self::term::Term;
