use std::thread;

use crate::consts;

/// Errai OS-level API.
pub struct System;

impl System {
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
}
