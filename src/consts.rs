use std::time::Duration;

use crate::core::ProcTable;

// -----------------------------------------------------------------------------
// Exit Codes
// -----------------------------------------------------------------------------

/// Execution success.
pub const E_CODE_SUCCESS: i32 = 0;

/// Initialization failure.
pub const E_CODE_FAILURE_INIT: i32 = -1;

/// Execution failure.
pub const E_CODE_FAILURE_EXEC: i32 = -2;

// -----------------------------------------------------------------------------
// System - Types
// -----------------------------------------------------------------------------

/// Maximum number of characters in an [`Atom`].
///
/// [`Atom`]: crate::core::Atom
pub const MAX_ATOM_CHARS: usize = 255;

/// Maximum number of [`Atom`]s in the atom table.
///
/// [`Atom`]: crate::core::Atom
pub const MAX_ATOM_COUNT: usize = 1 << 20;

// -----------------------------------------------------------------------------
// System - Process Behavior
// -----------------------------------------------------------------------------

/// Whether the [`ASYNC_DIST`] flag is set by default.
///
/// [`ASYNC_DIST`]: crate::erts::ProcessFlags::ASYNC_DIST
pub const SPAWN_INIT_ASYNC_DIST: bool = false;

/// Whether the [`TRAP_EXIT`] flag is set by default.
///
/// [`TRAP_EXIT`]: crate::erts::ProcessFlags::TRAP_EXIT
pub const SPAWN_INIT_TRAP_EXIT: bool = false;

// -----------------------------------------------------------------------------
// System - Scheduler Behavior
// -----------------------------------------------------------------------------

/// Default amount of parallelism the tokio runtime should use.
///
/// Note: This value is only used when a default value is not
///       retrievable from the host environment.
pub const DEFAULT_PARALLELISM: usize = 1;

/// Number of scheduler ticks before polling for external events.
pub const DEFAULT_EVENT_INTERVAL: u32 = 61;

/// Number of scheduler ticks before polling the global task queue.
pub const DEFAULT_GLOBAL_QUEUE_INTERVAL: u32 = 31;

/// Limit for additional threads spawned by the tokio runtime.
pub const DEFAULT_MAX_BLOCKING_THREADS: usize = 512;

/// Maximum number of I/O events processed per scheduler tick.
pub const DEFAULT_MAX_IO_EVENTS_PER_TICK: usize = 1024;

/// How long to keep threads in the blocking pool alive.
pub const DEFAULT_THREAD_KEEP_ALIVE: Duration = Duration::from_millis(10 * 1000);

/// Stack size (in bytes) for worker threads.
pub const DEFAULT_THREAD_STACK_SIZE: usize = 2 * 1024 * 1024;

// -----------------------------------------------------------------------------
// System - Shutdown
// -----------------------------------------------------------------------------

/// How long to wait for a clean shutdown of the internal runtime.
pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

// -----------------------------------------------------------------------------
// System - Memory Allocation
// -----------------------------------------------------------------------------

/// Number of pre-allocated entries in the process dictionary.
pub const CAP_PROC_DICTIONARY: usize = 8;

// Number of pre-allocated slots in the internal message buffer.
pub const CAP_PROC_MSG_BUFFER: usize = 8;

// Number of pre-allocated process states.
pub const CAP_REGISTERED_PROCS: usize = ProcTable::<()>::DEF_ENTRIES;

// Number of pre-allocated registered names.
pub const CAP_REGISTERED_NAMES: usize = ProcTable::<()>::MIN_ENTRIES;
