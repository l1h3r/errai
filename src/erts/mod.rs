//! Core ERTS (Errai Runtime System) types and APIs.
//!
//! This module provides the main user-facing types for process management,
//! messaging, and runtime control. It follows the BEAM terminology where
//! "ERTS" refers to the runtime system components.
//!
//! # Public API
//!
//! - [`Process`]: Main process API for spawning, messaging, and coordination
//! - [`ProcessFlags`]: Process behavior flags (trap_exit, async_dist)
//! - [`Runtime`]: Runtime control and system time
//! - [`Message`]: Message envelope for received messages
//! - [`ExitMessage`]: Trapped exit signal
//! - [`DownMessage`]: Monitor notification
//! - [`SpawnConfig`]: Process spawn configuration
//! - [`SpawnHandle`]: Handle to spawned process
//!
//! # Internal Components
//!
//! - Signal types: Internal signal protocol implementation
//! - Signal traits: Signal emission and reception
//!
//! # Module Organization
//!
//! - `message`: Message types and downcasting
//! - `process`: Process API implementation
//! - `runtime`: Runtime control functions
//! - `signal`: Signal protocol (internal)
//! - `spawn`: Spawn configuration types

mod message;
mod process;
mod runtime;
mod signal;
mod spawn;

pub(crate) use self::signal::ControlSignal;
pub(crate) use self::signal::MessageSignal;
pub(crate) use self::signal::Signal;
pub(crate) use self::signal::SignalDemonitor;
pub(crate) use self::signal::SignalEmit;
pub(crate) use self::signal::SignalExit;
pub(crate) use self::signal::SignalLink;
pub(crate) use self::signal::SignalLinkExit;
pub(crate) use self::signal::SignalMonitor;
pub(crate) use self::signal::SignalMonitorDown;
pub(crate) use self::signal::SignalRecv;
pub(crate) use self::signal::SignalSend;
pub(crate) use self::signal::SignalUnlink;
pub(crate) use self::signal::SignalUnlinkAck;

pub use self::message::DownMessage;
pub use self::message::DynMessage;
pub use self::message::ExitMessage;
pub use self::message::Message;
pub use self::process::Process;
pub use self::process::ProcessFlags;
pub use self::process::ProcessInfo;
pub use self::runtime::Runtime;
pub use self::spawn::SpawnConfig;
pub use self::spawn::SpawnHandle;
