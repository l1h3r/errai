// -----------------------------------------------------------------------------
// Local Name Registration
//
// BEAM Reference:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.h
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.c
// -----------------------------------------------------------------------------

use hashbrown::HashMap;
use hashbrown::hash_map::Entry;
use parking_lot::RwLockWriteGuard;

use crate::bifs::proc_with;
use crate::core::Atom;
use crate::core::InternalPid;
use crate::node::LocalNode;
use crate::proc::ProcExternal;
use crate::proc::ProcTask;
use crate::raise;

/// Registers a name for a process.
///
/// # Registration Protocol
///
/// 1. Validate name (not `undefined`)
/// 2. Acquire write lock on name table
/// 3. Check name is not already registered
/// 4. Check process is alive
/// 5. Check process doesn't have a registered name
/// 6. Register name → PID mapping
/// 7. Set process's registered name
///
/// # Panics
///
/// Raises exception if:
///
/// - Name is `undefined` (reserved)
/// - Name is already registered
/// - PID is not alive
/// - PID already has a registered name
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2295>
pub(crate) fn proc_register(this: &ProcTask, pid: InternalPid, name: Atom) {
  if name == Atom::UNDEFINED {
    raise!(Error, BadArg, "reserved name");
  }

  let mut name_guard: RwLockWriteGuard<'_, HashMap<Atom, InternalPid>> = LocalNode::names().write();

  let Entry::Vacant(name_entry) = name_guard.entry(name) else {
    raise!(Error, BadArg, "registered name");
  };

  proc_with(this, pid, |proc| {
    let Some(proc) = proc else {
      raise!(Error, BadArg, "not alive");
    };

    let mut proc_guard: RwLockWriteGuard<'_, ProcExternal> = proc.external.write();

    if proc_guard.name.is_none() {
      proc_guard.name = Some(name);
      name_entry.insert(proc.readonly.mpid);
    } else {
      raise!(Error, BadArg, "registered PID");
    }

    drop(proc_guard);
  });

  drop(name_guard);
}

/// Unregisters a name from a process.
///
/// # Unregistration Protocol
///
/// 1. Acquire write lock on name table
/// 2. Find PID for name (raise if not found)
/// 3. Clear process's registered name
/// 4. Remove name → PID mapping
///
/// # Panics
///
/// Raises exception if:
///
/// - Name is not registered
/// - PID's registered name doesn't match (inconsistent state)
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2309>
pub(crate) fn proc_unregister(this: &ProcTask, name: Atom) {
  let mut name_guard: RwLockWriteGuard<'_, HashMap<Atom, InternalPid>> = LocalNode::names().write();

  let Entry::Occupied(name_entry) = name_guard.entry(name) else {
    raise!(Error, BadArg, "unregistered name");
  };

  proc_with(this, *name_entry.get(), |proc| {
    let Some(proc) = proc else {
      name_entry.remove();
      return; // Ignore, the PID was just terminated
    };

    let mut proc_guard: RwLockWriteGuard<'_, ProcExternal> = proc.external.write();

    if proc_guard.name.is_some() {
      proc_guard.name = None;
      name_entry.remove();
    } else {
      raise!(Error, BadArg, "unregistered PID");
    }

    drop(proc_guard);
  });

  drop(name_guard);
}

/// Looks up a PID by registered name.
///
/// Returns `None` if the name is not registered. This is a read-only
/// operation that acquires a read lock.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2327>
pub(crate) fn proc_whereis(name: Atom) -> Option<InternalPid> {
  LocalNode::names().read().get(&name).copied()
}

/// Returns all currently registered names.
///
/// Returns a snapshot of registered names. The list may be stale immediately
/// after returning.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.c#L576>
pub(crate) fn proc_registered() -> Vec<Atom> {
  Vec::from_iter(LocalNode::names().read().keys().copied())
}
