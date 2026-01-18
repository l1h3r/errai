//! Runtime configuration constants and default values.
//!
//! This module defines the fundamental limits, default behaviors, and tuning
//! parameters for the Errai runtime system. These constants control everything
//! from atom table sizing to scheduler behavior to graceful shutdown timing.
//!
//! # Categories
//!
//! - **Exit Codes**: Process termination status codes
//! - **Type Limits**: Maximum sizes for atoms and tables
//! - **Process Behavior**: Default flag values at spawn time
//! - **Scheduler Behavior**: Tokio runtime tuning parameters
//! - **Shutdown**: Graceful termination timeouts
//! - **Memory Allocation**: Initial capacities for data structures

use std::time::Duration;

use crate::core::ProcTable;

// -----------------------------------------------------------------------------
// Exit Codes
// -----------------------------------------------------------------------------

/// Exit code indicating successful runtime initialization and execution.
///
/// Returned when the runtime completes normally without errors.
pub const E_CODE_SUCCESS: i32 = 0;

/// Exit code indicating a failure during runtime initialization.
///
/// Returned when the runtime fails to start, such as due to configuration
/// errors or resource allocation failures.
pub const E_CODE_FAILURE_INIT: i32 = -1;

/// Exit code indicating a failure during runtime execution.
///
/// Returned when the runtime encounters a fatal error during execution,
/// such as an unhandled panic in a critical system process.
pub const E_CODE_FAILURE_EXEC: i32 = -2;

// -----------------------------------------------------------------------------
// System - Types
// -----------------------------------------------------------------------------

/// Maximum number of characters allowed in an [`Atom`].
///
/// [`Atom`]: crate::core::Atom
pub const MAX_ATOM_CHARS: usize = 255;

/// Maximum number of bytes allowed in an [`Atom`].
///
/// This value assumes a worst-case of four bytes per Unicode scalar value
/// (UTF-8 encoding). The actual byte limit is `MAX_ATOM_CHARS * 4 = 1020` bytes.
///
/// Atoms exceeding this limit will cause an [`AtomTooLarge`] error.
///
/// [`Atom`]: crate::core::Atom
/// [`AtomTooLarge`]: crate::core::AtomTableError::AtomTooLarge
pub const MAX_ATOM_BYTES: usize = MAX_ATOM_CHARS.strict_mul(4);

/// Maximum number of [`Atom`]s that can be stored in the atom table.
///
/// The atom table is limited to 1,048,576 (2²⁰) distinct atoms. This
/// prevents unbounded memory growth from dynamic atom creation.
///
/// Exceeding this limit will cause a [`TooManyAtoms`] error.
///
/// [`Atom`]: crate::core::Atom
/// [`TooManyAtoms`]: crate::core::AtomTableError::TooManyAtoms
pub const MAX_ATOM_COUNT: usize = 1 << 20;

// -----------------------------------------------------------------------------
// System - Process Behavior
// -----------------------------------------------------------------------------

/// Default state of the [`ASYNC_DIST`] process flag at spawn time.
///
/// When `false` (default), distributed messages are sent synchronously.
/// When `true`, distributed messages are sent asynchronously.
///
/// This can be overridden per-process via [`SpawnConfig`].
///
/// [`ASYNC_DIST`]: crate::erts::ProcessFlags::ASYNC_DIST
/// [`SpawnConfig`]: crate::erts::SpawnConfig
pub const SPAWN_INIT_ASYNC_DIST: bool = false;

/// Default state of the [`TRAP_EXIT`] process flag at spawn time.
///
/// When `false` (default), abnormal exit signals cause the process to
/// terminate. When `true`, exit signals are converted to messages.
///
/// This can be overridden per-process via [`SpawnConfig`].
///
/// [`TRAP_EXIT`]: crate::erts::ProcessFlags::TRAP_EXIT
/// [`SpawnConfig`]: crate::erts::SpawnConfig
pub const SPAWN_INIT_TRAP_EXIT: bool = false;

// -----------------------------------------------------------------------------
// System - Scheduler Behavior
// -----------------------------------------------------------------------------

/// Default parallelism used when host CPU information is unavailable.
///
/// This value determines the number of Tokio worker threads created when
/// the system cannot detect CPU count. In practice, CPU detection usually
/// succeeds, making this a fallback value.
pub const DEFAULT_PARALLELISM: usize = 1;

/// Scheduler ticks between polling for external events.
///
/// The scheduler checks for I/O events and external wakeups every 61 ticks.
/// Lower values improve responsiveness but increase polling overhead. Higher
/// values reduce overhead but may delay event processing.
pub const DEFAULT_EVENT_INTERVAL: u32 = 61;

/// Scheduler ticks between polling the global task queue.
///
/// The scheduler checks the global queue every 31 ticks to balance fairness
/// between local and global tasks. Lower values improve fairness but increase
/// contention. Higher values favor local tasks but may starve global tasks.
pub const DEFAULT_GLOBAL_QUEUE_INTERVAL: u32 = 31;

/// Maximum number of additional blocking threads spawned by the runtime.
///
/// Tokio spawns blocking threads on-demand when blocking operations are
/// performed. This limit prevents unbounded thread creation under heavy
/// blocking workloads.
///
/// Note that this is in addition to the core worker threads.
pub const DEFAULT_MAX_BLOCKING_THREADS: usize = 512;

/// Maximum number of I/O events processed per scheduler tick.
///
/// The scheduler processes up to this many I/O events per tick before
/// returning to task execution. This bounds the cost of I/O processing
/// and ensures tasks aren't starved by heavy I/O activity.
pub const DEFAULT_MAX_IO_EVENTS_PER_TICK: usize = 1024;

/// Duration that idle blocking threads are kept alive.
///
/// Blocking threads that remain idle for longer than this duration are
/// eligible for termination. This helps reclaim resources during periods
/// of low blocking activity.
pub const DEFAULT_THREAD_KEEP_ALIVE: Duration = Duration::from_millis(10 * 1000);

/// Stack size allocated for each Tokio worker thread.
///
/// This value applies to both async worker threads and blocking task threads.
/// The default of 2 MiB balances stack overflow safety with memory efficiency.
pub const DEFAULT_THREAD_STACK_SIZE: usize = 2 * 1024 * 1024;

// -----------------------------------------------------------------------------
// System - Shutdown
// -----------------------------------------------------------------------------

/// Maximum duration allowed for graceful runtime shutdown.
///
/// The runtime waits up to this duration for processes to terminate cleanly
/// during shutdown.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

// -----------------------------------------------------------------------------
// System - Memory Allocation
// -----------------------------------------------------------------------------

/// Initial capacity of the per-process dictionary.
pub const CAP_PROC_DICTIONARY: usize = 8;

/// Initial capacity of the per-process internal message buffer.
pub const CAP_PROC_MSG_BUFFER: usize = 8;

/// Number of process slots allocated in the global process table.
pub const MAX_REGISTERED_PROCS: usize = ProcTable::<()>::DEF_ENTRIES;

/// Initial capacity of the process name registry.
pub const CAP_REGISTERED_NAMES: usize = ProcTable::<()>::MIN_ENTRIES;
