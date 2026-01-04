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
      async_dist: false,
      trap_exit: false,
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
