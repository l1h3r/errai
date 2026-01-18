//! Process identifier types for local and distributed processes.
//!
//! This module provides process identifiers (PIDs) that uniquely name processes
//! both locally and across distributed nodes. PIDs are the fundamental addressing
//! mechanism for process-to-process communication.
//!
//! # PID Types
//!
//! - [`InternalPid`]: Local process identifier (64-bit)
//! - [`ExternalPid`]: Distributed process identifier (local PID + node)
//! - [`ProcessId`]: Trait abstracting over both types
//!
//! # Local vs Distributed
//!
//! - **Local PIDs** ([`InternalPid`]) identify processes on the current node
//! - **External PIDs** ([`ExternalPid`]) identify processes on remote nodes
//!
//! The [`ProcessId`] trait enables writing code that works with both types.
//!
//! # PID Format
//!
//! PIDs display in Erlang-compatible format:
//!
//! - Local: `#PID<0.Number.Serial>`
//! - Remote: `#PID<Node.Number.Serial>`

mod external;
mod internal;
mod traits;

pub use self::external::ExternalPid;
pub use self::internal::InternalPid;
pub use self::traits::ProcessId;
