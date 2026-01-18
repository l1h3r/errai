//! Internal process data structures and signal queue implementation.
//!
//! This module contains the low-level implementation of process state,
//! message queues, and inter-process communication primitives. These types
//! are internal to the runtime and not exposed to user code.
//!
//! # Architecture
//!
//! Process data is split into three sections with different locking requirements:
//!
//! - [`ProcReadOnly`]: Immutable data accessible without locks
//! - [`ProcInternal`]: Mutable state protected by [`Mutex`]
//! - [`ProcExternal`]: Rarely-modified state protected by [`RwLock`]
//!
//! This separation minimizes lock contention by isolating frequently-accessed
//! read-only data from mutable state.
//!
//! # Signal Queue
//!
//! The signal queue ([`ProcSend`]/[`ProcRecv`]) uses Tokio's unbounded MPSC
//! channel for inter-process signal delivery. The mailbox ([`ProcMail`])
//! provides selective receive with user-defined filter functions.
//!
//! # Lifetime Management
//!
//! [`ProcTask`] wraps process data and triggers cleanup on drop, ensuring
//! processes are properly removed from the process table when their task
//! completes.
//!
//! [`Mutex`]: ::parking_lot::Mutex
//! [`RwLock`]: ::parking_lot::RwLock

mod proc_data;
mod proc_dict;
mod proc_link;
mod proc_task;
mod sig_queue;

pub(crate) use self::proc_data::ProcData;
pub(crate) use self::proc_data::ProcExternal;
pub(crate) use self::proc_data::ProcInternal;
pub(crate) use self::proc_data::ProcReadOnly;
pub(crate) use self::proc_dict::ProcDict;
pub(crate) use self::proc_link::ProcLink;
pub(crate) use self::proc_link::ProcMonitor;
pub(crate) use self::proc_task::ProcTask;
pub(crate) use self::sig_queue::ProcMail;
pub(crate) use self::sig_queue::ProcRecv;
pub(crate) use self::sig_queue::ProcSend;
pub(crate) use self::sig_queue::WeakProcSend;
pub(crate) use self::sig_queue::unbounded_channel;
