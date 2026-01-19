//! Errai - A BEAM-inspired fault-tolerant async runtime for Rust.
//!
//! Errai provides Erlang/BEAM-style process behavior and interactions,
//! including process-to-process communication, crash detection with links
//! and monitors, and selective message reception.
//!
//! # Quick Start
//!
//! ```no_run
//! use errai::erts::Process;
//! use errai::init;
//!
//! init::block_on(async {
//!   let pid = Process::spawn(async {
//!     println!("Hello from process!");
//!   });
//!
//!   Process::send(pid, "Hello!");
//! });
//! ```
//!
//! # Core Modules
//!
//! - [`init`]: Runtime initialization and entry point
//! - [`erts`]: Process API and runtime control
//! - [`core`]: Core types (PIDs, atoms, terms, references)
//! - [`error`]: Exception system
//! - [`consts`]: Runtime configuration constants

mod bifs;
mod loom;
mod proc;
mod utils;

pub mod consts;
pub mod core;
pub mod error;
pub mod erts;
pub mod init;
pub mod node;
pub mod tyre;
