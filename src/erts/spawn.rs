use crate::consts::SPAWN_INIT_ASYNC_DIST;
use crate::consts::SPAWN_INIT_TRAP_EXIT;
use crate::core::InternalPid;
use crate::core::MonitorRef;

// -----------------------------------------------------------------------------
// Spawn Config
// -----------------------------------------------------------------------------

/// Configuration options for spawning a new process.
///
/// Controls initial process state including links, monitors, and flags.
///
/// # Default Values
///
/// - `link`: `false`
/// - `monitor`: `false`
/// - `async_dist`: [`SPAWN_INIT_ASYNC_DIST`]
/// - `trap_exit`: [`SPAWN_INIT_TRAP_EXIT`]
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SpawnConfig {
  /// Creates a link to the parent process.
  ///
  /// Equivalent to calling [`Process::spawn_link()`].
  ///
  /// [`Process::spawn_link()`]: crate::erts::Process::spawn_link
  pub link: bool,
  /// Monitors the new process.
  ///
  /// Equivalent to calling [`Process::spawn_monitor()`].
  ///
  /// [`Process::spawn_monitor()`]: crate::erts::Process::spawn_monitor
  pub monitor: bool,
  /// Sets the [`ASYNC_DIST`] flag on the spawned process.
  ///
  /// Controls whether distributed messages are sent asynchronously.
  ///
  /// [`ASYNC_DIST`]: crate::erts::ProcessFlags::ASYNC_DIST
  pub async_dist: bool,
  /// Sets the [`TRAP_EXIT`] flag on the spawned process.
  ///
  /// When enabled, EXIT signals are converted to messages instead of
  /// causing termination.
  ///
  /// [`TRAP_EXIT`]: crate::erts::ProcessFlags::TRAP_EXIT
  pub trap_exit: bool,
}

impl SpawnConfig {
  /// Creates a new spawn configuration with default values.
  #[inline]
  pub const fn new() -> Self {
    Self {
      link: false,
      monitor: false,
      async_dist: SPAWN_INIT_ASYNC_DIST,
      trap_exit: SPAWN_INIT_TRAP_EXIT,
    }
  }

  /// Creates a spawn configuration with linking enabled.
  #[inline]
  pub const fn new_link() -> Self {
    let mut this: Self = Self::new();
    this.link = true;
    this
  }

  /// Creates a spawn configuration with monitoring enabled.
  #[inline]
  pub const fn new_monitor() -> Self {
    let mut this: Self = Self::new();
    this.monitor = true;
    this
  }
}

impl Default for SpawnConfig {
  #[inline]
  fn default() -> Self {
    Self::new()
  }
}

// -----------------------------------------------------------------------------
// Spawn Handle
// -----------------------------------------------------------------------------

/// A handle to a spawned process.
///
/// Returned from spawn operations to identify the new process. The variant
/// depends on whether the process was spawned with monitoring enabled.
///
/// # Variants
///
/// - [`Process`]: Normal spawn or spawn with link
/// - [`Monitor`]: Spawn with monitor
///
/// [`Process`]: Self::Process
/// [`Monitor`]: Self::Monitor
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpawnHandle {
  /// A process spawned without monitoring.
  Process(InternalPid),
  /// A process spawned with monitoring enabled.
  ///
  /// Contains both the PID and the monitor reference.
  Monitor(InternalPid, MonitorRef),
}

impl SpawnHandle {
  /// Returns `true` if this is a normal process handle.
  #[inline]
  pub const fn is_process(&self) -> bool {
    matches!(self, Self::Process(_))
  }

  /// Returns `true` if this is a monitored process handle.
  #[inline]
  pub const fn is_monitor(&self) -> bool {
    matches!(self, Self::Monitor(_, _))
  }
}
