//! Core runtime types and tables for the Errai system.
//!
//! This module provides the foundational type system and data structures
//! of the Errai runtime: process identifiers, message destinations, references,
//! atoms, terms, and the core tables for atom interning and process storage.
//!
//! # Module Organization
//!
//! - **Types**: Process IDs, references, atoms, terms, destinations
//! - **Tables**: Atom interning and process storage
//!
//! # Core Tables
//!
//! ## Atom Table
//!
//! The [`AtomTable`] provides global string interning with permanent storage.
//! Once interned, atoms are never deallocated and can be compared by their
//! 32-bit slot index.
//!
//! ## Process Table
//!
//! The [`ProcTable`] provides lock-free, cache-aware storage for process data.
//! It supports concurrent insertion, removal, and lookup without traditional
//! locks, using atomic operations and tagged pointers.
//!
//! # Common Imports
//!
//! Most code will import types directly from this module:
//!
//! ```
//! use errai::core::{
//!   Atom, Term, Exit,
//!   InternalPid, ExternalPid,
//!   InternalDest, ExternalDest,
//!   MonitorRef, InternalRef,
//! };
//! ```
//!
//! # Core Concepts
//!
//! ## Process Addressing
//!
//! Processes are addressed using PIDs ([`InternalPid`], [`ExternalPid`]) or
//! registered names wrapped in destination types ([`InternalDest`],
//! [`ExternalDest`]).
//!
//! ## String Interning
//!
//! The [`Atom`] type provides efficient string interning with O(1) equality
//! comparison. All atoms are permanently stored in the global [`AtomTable`].
//!
//! ## Dynamic Typing
//!
//! The [`Term`] type enables type-erased values that can be sent between
//! processes. Values can be safely recovered using downcasting.
//!
//! ## Fault Handling
//!
//! The [`Exit`] type represents process termination reasons. Exit reasons
//! propagate through links and monitors to enable supervision patterns.
//!
//! # Examples
//!
//! ```
//! use errai::core::{Atom, InternalPid, InternalDest, Exit, ProcTable};
//!
//! // Create an atom
//! let name = Atom::new("my_process");
//!
//! // Create a destination from a name
//! let dest = InternalDest::from(name);
//!
//! // Create a process table
//! let table = ProcTable::<u64>::new();
//!
//! // Check exit reasons
//! if Exit::NORMAL.is_normal() {
//!   println!("Clean shutdown");
//! }
//! ```

mod table;
mod types;

pub use self::table::AtomTable;
pub use self::table::AtomTableError;
pub use self::table::ProcTable;
pub use self::table::ProcTableFull;
pub use self::table::ProcTableKeys;

pub use self::types::AliasRef;
pub use self::types::Atom;
pub use self::types::Exit;
pub use self::types::ExternalDest;
pub use self::types::ExternalPid;
pub use self::types::ExternalRef;
pub use self::types::InternalDest;
pub use self::types::InternalPid;
pub use self::types::InternalRef;
pub use self::types::Item;
pub use self::types::MonitorRef;
pub use self::types::ProcessId;
pub use self::types::Term;
pub use self::types::TimerRef;
