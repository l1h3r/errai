use crate::erts::Runtime;
use crate::lang::InternalPid;
use crate::lang::MonitorRef;

// -----------------------------------------------------------------------------
// Spawn Config
// -----------------------------------------------------------------------------

/// Options used to configure a spawned process.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SpawnConfig {
  /// Creates a link to the parent process.
  ///
  /// This is the same as calling [`Process::spawn_link`].
  ///
  /// [`Process::spawn_link`]: crate::erts::Process::spawn_link
  pub link: bool,
  /// Monitors the new process.
  ///
  /// This is the same as calling [`Process::spawn_monitor`].
  ///
  /// [`Process::spawn_monitor`]: crate::erts::Process::spawn_monitor
  pub monitor: bool,
  /// Sets the [`ASYNC_DIST`] process flag of the spawned process.
  ///
  /// [`ASYNC_DIST`]: crate::erts::ProcessFlags::ASYNC_DIST
  pub async_dist: bool,
  /// Sets the [`TRAP_EXIT`] process flag of the spawned process.
  ///
  /// [`TRAP_EXIT`]: crate::erts::ProcessFlags::TRAP_EXIT
  pub trap_exit: bool,
}

impl SpawnConfig {
  #[inline]
  pub const fn new() -> Self {
    Self {
      link: false,
      monitor: false,
      async_dist: Runtime::SPAWN_INIT_ASYNC_DIST,
      trap_exit: Runtime::SPAWN_INIT_TRAP_EXIT,
    }
  }

  #[inline]
  pub const fn new_link() -> Self {
    let mut this: Self = Self::new();
    this.link = true;
    this
  }

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
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpawnHandle {
  /// A normal process.
  Process(InternalPid),
  /// A monitored process.
  Monitor(InternalPid, MonitorRef),
}

impl SpawnHandle {
  /// Returns `true` if the spawn handle is a normal process.
  #[inline]
  pub const fn is_process(&self) -> bool {
    matches!(self, Self::Process(_))
  }

  /// Returns `true` if the spawn handle is a monitored process.
  #[inline]
  pub const fn is_monitor(&self) -> bool {
    matches!(self, Self::Monitor(_, _))
  }
}
