use hashbrown::HashMap;

use crate::lang::Atom;
use crate::lang::InternalPid;
use crate::lang::Term;

/// Information about a process.
#[derive(Debug)]
pub struct ProcessInfo {
  /// The value of the [`ASYNC_DIST`] process flag.
  ///
  /// [`ASYNC_DIST`]: crate::erts::ProcessFlags::ASYNC_DIST
  pub async_dist: bool,
  /// The process dictionary.
  pub dictionary: HashMap<Atom, Term>,
  /// The group leader for the I/O of the process.
  pub pid_group_leader: InternalPid,
  /// The PID of the parent process (the one that spawned this process).
  ///
  /// Note: Every process has a parent PID, except the root process.
  pub pid_spawn_parent: Option<InternalPid>,
  /// The name registered to this process, if any.
  pub registered_name: Option<Atom>,
  /// The value of the [`TRAP_EXIT`] process flag.
  ///
  /// [`TRAP_EXIT`]: crate::erts::ProcessFlags::TRAP_EXIT
  pub trap_exit: bool,
}
