use std::time::Duration;
use std::time::SystemTime;

/// Errai runtime system API.
///
/// Provides functions for runtime control (halt, stop) and system time
/// queries.
pub struct Runtime;

impl Runtime {
  // ---------------------------------------------------------------------------
  // Runtime API
  // ---------------------------------------------------------------------------

  /// Forcefully stops the Errai runtime system.
  ///
  /// This immediately terminates the runtime without graceful shutdown.
  /// Use [`stop()`] for graceful termination.
  ///
  /// # Status Codes
  ///
  /// The `status` parameter determines the process exit code.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#halt/2>
  ///
  /// [`stop()`]: Runtime::stop
  pub fn halt(_status: u8) {
    todo!()
  }

  /// Gracefully stops the Errai runtime system.
  ///
  /// This initiates graceful shutdown, allowing processes to clean up.
  /// Use [`halt()`] for immediate termination.
  ///
  /// # Status Codes
  ///
  /// The `status` parameter determines the process exit code.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/init.html#stop/1>
  ///
  /// [`halt()`]: Runtime::halt
  pub fn stop(_status: u8) {
    todo!()
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
