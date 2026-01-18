//! Message destination types supporting local and distributed addressing.
//!
//! This module provides destination types for addressing processes in
//! send operations. Destinations can target either direct process identifiers
//! or registered names, both locally and across distributed nodes.
//!
//! # Destination Types
//!
//! - [`InternalDest`]: Local-only addressing (PID or name)
//! - [`ExternalDest`]: Distributed addressing (local or remote)
//!
//! # Addressing Modes
//!
//! Processes can be addressed in four ways:
//!
//! 1. **Local PID**: Direct reference to a local process
//! 2. **Local name**: Registered name on the local node
//! 3. **Remote PID**: Direct reference to a remote process
//! 4. **Remote name**: Registered name on a remote node
//!
//! # Name Registration
//!
//! Registered names provide stable addresses for processes that may restart
//! or migrate. Names are looked up at send time, allowing late binding.

mod external;
mod internal;

pub use self::external::ExternalDest;
pub use self::internal::InternalDest;
