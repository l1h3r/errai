use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use crate::consts;

// -----------------------------------------------------------------------------
// Runtime
// -----------------------------------------------------------------------------

/// Errai runtime system API.
pub struct Runtime;

impl Runtime {
  // ---------------------------------------------------------------------------
  // Runtime API
  // ---------------------------------------------------------------------------

  /// Returns the number of available CPU cores.
  ///
  /// Falls back to [`DEFAULT_PARALLELISM`] if CPU detection fails.
  ///
  /// [`DEFAULT_PARALLELISM`]: consts::DEFAULT_PARALLELISM
  pub fn available_cpus() -> usize {
    match thread::available_parallelism() {
      Ok(count) => count.get(),
      Err(_) => consts::DEFAULT_PARALLELISM,
    }
  }

  /// Returns the current OS system time as a POSIX duration.
  ///
  /// This returns the duration since the Unix epoch (January 1, 1970 UTC).
  /// If system time is before the epoch, returns [`Duration::ZERO`].
  ///
  /// REF: <https://www.erlang.org/doc/apps/kernel/os.html#system_time/0>
  pub fn time() -> Duration {
    SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap_or(Duration::ZERO)
  }
}

// -----------------------------------------------------------------------------
// Runtime Config
// -----------------------------------------------------------------------------

pub struct RuntimeConfig {
  // ---------------------------------------------------------------------------
  // Tokio Runtime Configuration
  // ---------------------------------------------------------------------------
  pub rt_event_interval: u32,
  pub rt_global_queue_interval: u32,
  pub rt_max_blocking_threads: usize,
  pub rt_max_io_events_per_tick: usize,
  pub rt_shutdown_timeout: Duration,
  pub rt_thread_keep_alive: Duration,
  pub rt_thread_stack_size: usize,
  pub rt_worker_threads: usize,
  // ---------------------------------------------------------------------------
  // Tracing Subscriber Configuration
  // ---------------------------------------------------------------------------
  pub tracing_source_file: bool,
  pub tracing_source_line: bool,
  pub tracing_source_name: bool,
  pub tracing_thread_info: bool,
  pub tracing_verbose: bool,
  pub tracing_very_verbose: bool,
}

impl RuntimeConfig {
  #[inline]
  pub fn new() -> Self {
    Self {
      rt_event_interval: consts::DEFAULT_EVENT_INTERVAL,
      rt_global_queue_interval: consts::DEFAULT_GLOBAL_QUEUE_INTERVAL,
      rt_max_blocking_threads: consts::DEFAULT_MAX_BLOCKING_THREADS,
      rt_max_io_events_per_tick: consts::DEFAULT_MAX_IO_EVENTS_PER_TICK,
      rt_shutdown_timeout: consts::SHUTDOWN_TIMEOUT_RUNTIME,
      rt_thread_keep_alive: consts::DEFAULT_THREAD_KEEP_ALIVE,
      rt_thread_stack_size: consts::DEFAULT_THREAD_STACK_SIZE,
      rt_worker_threads: Runtime::available_cpus(),
      tracing_source_file: false,
      tracing_source_line: false,
      tracing_source_name: false,
      tracing_thread_info: true,
      tracing_verbose: true,
      tracing_very_verbose: false,
    }
  }

  #[inline]
  pub const fn tracing_filter(&self) -> tracing::Level {
    if self.tracing_very_verbose {
      tracing::Level::TRACE
    } else if self.tracing_verbose {
      tracing::Level::DEBUG
    } else {
      tracing::Level::INFO
    }
  }
}

impl Default for RuntimeConfig {
  #[inline]
  fn default() -> Self {
    Self::new()
  }
}
