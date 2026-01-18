//! Message types for inter-process communication.
//!
//! This module provides the message types used in process mailboxes and
//! selective receive operations. Messages can be regular terms or special
//! system messages (EXIT, DOWN).
//!
//! # Message Types
//!
//! - [`Message`]: Generic message container (term, exit, or down)
//! - [`DynMessage`]: Type alias for [`Message<Term>`]
//! - [`ExitMessage`]: Trapped exit signal from a linked process
//! - [`DownMessage`]: Monitor notification for a watched process
//!
//! # Type Checking
//!
//! Messages support runtime type checking via [`Message::is()`] and
//! [`Message::is_exact()`], enabling selective receive based on message
//! type.
//!
//! [`Message::is()`]: Message::is
//! [`Message::is_exact()`]: Message::is_exact

mod base;
mod down;
mod exit;

pub use self::base::DynMessage;
pub use self::base::Message;
pub use self::down::DownMessage;
pub use self::exit::ExitMessage;
