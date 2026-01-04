use crate::erts::MonitorRef;
use crate::lang::InternalPid;

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
