use std::time::Duration;

/// Errai runtime API.
pub struct Runtime;

impl Runtime {
  /// Forcefully stops the Errai runtime system.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#halt/2>
  pub fn halt(status: u8) {
    todo!()
  }

  /// Gracefully stops the Errai runtime system.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/init.html#stop/1>
  pub fn stop(status: u8) {
    todo!()
  }

  /// Returns the current OS system time as a POSIX time duration.
  ///
  /// REF: <https://www.erlang.org/doc/apps/kernel/os.html#system_time/0>
  pub fn time() -> Duration {
    todo!()
  }
}
