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
// System - Erlang Things
// -----------------------------------------------------------------------------

/// Erlang constant: `FUNNY_NUMBER7`.
///
/// REF: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_term_hashing.c#L103>
pub const ERL_FUNNY_NUMBER7: u64 = 268438039;

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
pub const MAX_ATOM_BYTES: usize = MAX_ATOM_CHARS.strict_mul(size_of::<char>());

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
/// The scheduler checks for I/O events and external wakeups every 255 ticks.
/// Lower values improve responsiveness but increase polling overhead. Higher
/// values reduce overhead but may delay event processing.
pub const DEFAULT_EVENT_INTERVAL: u32 = 255;

/// Scheduler ticks between polling the global task queue.
///
/// The scheduler checks the global queue every 127 ticks to balance fairness
/// between local and global tasks. Lower values improve fairness but increase
/// contention. Higher values favor local tasks but may starve global tasks.
pub const DEFAULT_GLOBAL_QUEUE_INTERVAL: u32 = 127;

/// Maximum number of additional blocking threads spawned by the runtime.
///
/// Tokio spawns blocking threads on-demand when blocking operations are
/// performed. This limit prevents unbounded thread creation under heavy
/// blocking workloads.
///
/// Note that this is in addition to the core worker threads.
pub const DEFAULT_MAX_BLOCKING_THREADS: usize = 4;

/// Maximum number of I/O events processed per scheduler tick.
///
/// The scheduler processes up to this many I/O events per tick before
/// returning to task execution. This bounds the cost of I/O processing
/// and ensures tasks aren't starved by heavy I/O activity.
pub const DEFAULT_MAX_IO_EVENTS_PER_TICK: usize = 64;

/// Duration that idle blocking threads are kept alive.
///
/// Blocking threads that remain idle for longer than this duration are
/// eligible for termination. This helps reclaim resources during periods
/// of low blocking activity.
pub const DEFAULT_THREAD_KEEP_ALIVE: Duration = Duration::from_secs(60);

/// Stack size allocated for each Tokio worker thread.
///
/// This value applies to both async worker threads and blocking task threads.
pub const DEFAULT_THREAD_STACK_SIZE: usize = 1024 * 1024;

// -----------------------------------------------------------------------------
// System - Shutdown
// -----------------------------------------------------------------------------

/// Maximum duration allowed for graceful runtime shutdown.
///
/// The runtime waits up to this duration for processes to terminate cleanly
/// during shutdown.
pub const SHUTDOWN_TIMEOUT_RUNTIME: Duration = Duration::from_secs(30);

/// Maximum duration allowed for graceful timer service wheel worker shutdown.
///
/// Each timer service wheel worker waits up to this duration for tasks to
/// terminate cleanly during shutdown.
pub const SHUTDOWN_TIMEOUT_WHEEL_WORKER: Duration = Duration::from_millis(200);

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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use crate::consts::*;

  #[test]
  fn test_max_atom_count_is_power_of_two() {
    assert!(MAX_ATOM_COUNT.is_power_of_two());
  }

  #[test]
  fn test_scheduler_intervals_are_reasonable() {
    assert!(DEFAULT_EVENT_INTERVAL > 0);
    assert!(DEFAULT_EVENT_INTERVAL < 1000);

    assert!(DEFAULT_GLOBAL_QUEUE_INTERVAL > 0);
    assert!(DEFAULT_GLOBAL_QUEUE_INTERVAL < DEFAULT_EVENT_INTERVAL);
  }

  #[test]
  fn test_max_blocking_threads_is_reasonable() {
    assert!(DEFAULT_MAX_BLOCKING_THREADS > 0);
    assert!(DEFAULT_MAX_BLOCKING_THREADS <= 1024);
  }

  #[test]
  fn test_thread_stack_size_is_reasonable() {
    assert!(DEFAULT_THREAD_STACK_SIZE >= 1024 * 1024);
    assert!(DEFAULT_THREAD_STACK_SIZE <= 1024 * 1024 * 8);
  }

  #[test]
  fn test_shutdown_timeout_is_reasonable() {
    assert!(SHUTDOWN_TIMEOUT_RUNTIME.as_secs() >= 5);
    assert!(SHUTDOWN_TIMEOUT_RUNTIME.as_secs() <= 300);

    assert!(SHUTDOWN_TIMEOUT_WHEEL_WORKER.as_millis() >= 25);
    assert!(SHUTDOWN_TIMEOUT_WHEEL_WORKER.as_millis() <= 500);
  }

  #[test]
  fn test_capacity_constants_are_powers_of_two() {
    assert!(CAP_PROC_DICTIONARY.is_power_of_two());
    assert!(CAP_PROC_MSG_BUFFER.is_power_of_two());
    assert!(MAX_REGISTERED_PROCS.is_power_of_two());
    assert!(CAP_REGISTERED_NAMES.is_power_of_two());
  }
}
