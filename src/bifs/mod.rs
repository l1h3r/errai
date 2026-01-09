use hashbrown::HashMap;
use hashbrown::hash_map::Entry;
use parking_lot::RwLock;
use parking_lot::RwLockReadGuard;
use parking_lot::RwLockWriteGuard;
use std::sync::LazyLock;
use triomphe::Arc;

use crate::erts::ProcessData;
use crate::erts::ProcessSlot;
use crate::erts::ProcessTable;
use crate::erts::ProcessTask;
use crate::lang::Atom;
use crate::lang::InternalPid;
use crate::lang::RawPid;
use crate::lang::Term;

// The number of pre-allocated process states.
const CAP_REGISTERED_PROCS: usize = ProcessTable::<ProcessSlot>::DEF_ENTRIES;

// The number of pre-allocated registered names.
const CAP_REGISTERED_NAMES: usize = ProcessTable::<ProcessSlot>::MIN_ENTRIES;

// A table mapping internal process identifiers to process data.
static REGISTERED_PROCS: LazyLock<ProcessTable<ProcessSlot>> =
  LazyLock::new(|| ProcessTable::with_capacity(CAP_REGISTERED_PROCS));

// A table mapping registered names to internal process identifiers.
static REGISTERED_NAMES: LazyLock<RwLock<HashMap<Atom, InternalPid>>> =
  LazyLock::new(|| RwLock::new(HashMap::with_capacity(CAP_REGISTERED_NAMES)));

// -----------------------------------------------------------------------------
// Error Utilities
// -----------------------------------------------------------------------------

macro_rules! raise {
  ($error:ident, $type:literal, $data:literal) => {
    ::std::panic!("{}", $crate::core::$error::new($type, $data));
  };
}

// -----------------------------------------------------------------------------
// Common Utilities
// -----------------------------------------------------------------------------

pub(crate) fn translate_pid(pid: RawPid) -> Option<(u32, u32)> {
  if pid.into_bits() & RawPid::TAG_MASK == RawPid::TAG_DATA {
    Some(REGISTERED_PROCS.translate_pid(pid))
  } else {
    None
  }
}

// -----------------------------------------------------------------------------
// Local Name Registration
// -----------------------------------------------------------------------------

pub(crate) fn process_register(process: &ProcessTask, pid: InternalPid, name: Atom) {
  // ---------------------------------------------------------------------------
  // 1. Validate
  // ---------------------------------------------------------------------------

  if name == Atom::UNDEFINED {
    raise!(ArgumentError, "badarg", "register/2 - reserved name");
  }

  // ---------------------------------------------------------------------------
  // 2. Lock Name Registry
  // ---------------------------------------------------------------------------

  let mut name_guard: RwLockWriteGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.write();

  // ---------------------------------------------------------------------------
  // 3. Find Entry
  // ---------------------------------------------------------------------------

  let Entry::Vacant(name_entry) = name_guard.entry(name) else {
    raise!(ArgumentError, "badarg", "register/2 - registered name");
  };

  // ---------------------------------------------------------------------------
  // 3. Find Process
  // ---------------------------------------------------------------------------

  let hold: Arc<ProcessSlot>;
  let this: &ProcessSlot;

  if process.root.id == pid {
    this = process;
  } else {
    let Some(context) = REGISTERED_PROCS.get(pid.bits()) else {
      raise!(ArgumentError, "badarg", "register/2 - PID not alive");
    };

    hold = context;
    this = &hold;
  }

  // ---------------------------------------------------------------------------
  // 4. Lock Process
  // ---------------------------------------------------------------------------

  let mut proc_guard: RwLockWriteGuard<'_, ProcessData> = this.data.write();

  // ---------------------------------------------------------------------------
  // 4. Update Process/Registry
  // ---------------------------------------------------------------------------

  if proc_guard.name.is_some() {
    raise!(ArgumentError, "badarg", "register/2 - registered PID");
  }

  proc_guard.name = Some(name);
  name_entry.insert(pid);

  // ---------------------------------------------------------------------------
  // 5. Unlock Process/Registry
  // ---------------------------------------------------------------------------

  drop(proc_guard);
  drop(name_guard);
}

pub(crate) fn process_unregister(process: &ProcessTask, name: Atom) {
  // ---------------------------------------------------------------------------
  // 1. Lock Name Registry
  // ---------------------------------------------------------------------------

  let mut name_guard: RwLockWriteGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.write();

  // ---------------------------------------------------------------------------
  // 2. Find Entry
  // ---------------------------------------------------------------------------

  let Entry::Occupied(name_entry) = name_guard.entry(name) else {
    raise!(ArgumentError, "badarg", "unregister/1 - unregistered name");
  };

  // ---------------------------------------------------------------------------
  // 3. Find Process
  // ---------------------------------------------------------------------------

  let hold: Arc<ProcessSlot>;
  let this: &ProcessSlot;

  if process.root.id == *name_entry.get() {
    this = process;
  } else {
    let Some(context) = REGISTERED_PROCS.get(name_entry.get().bits()) else {
      // TODO: This should be unreachable (?)
      raise!(ArgumentError, "badarg", "unregister/1 - PID not alive");
    };

    hold = context;
    this = &hold;
  }

  // ---------------------------------------------------------------------------
  // 4. Lock Process
  // ---------------------------------------------------------------------------

  let mut proc_guard: RwLockWriteGuard<'_, ProcessData> = this.data.write();

  // ---------------------------------------------------------------------------
  // 4. Update Process/Registry
  // ---------------------------------------------------------------------------

  proc_guard.name = None;
  name_entry.remove();

  // ---------------------------------------------------------------------------
  // 5. Unlock Process/Registry
  // ---------------------------------------------------------------------------

  drop(proc_guard);
  drop(name_guard);
}

pub(crate) fn process_whereis(name: Atom) -> Option<InternalPid> {
  let guard: RwLockReadGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.read();
  let value: Option<InternalPid> = guard.get(&name).copied();

  drop(guard);

  value
}

pub(crate) fn process_registered() -> Vec<Atom> {
  let guard: RwLockReadGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.read();
  let value: Vec<Atom> = Vec::from_iter(guard.keys().copied());

  drop(guard);

  value
}

// -----------------------------------------------------------------------------
// Process Dictionary
// -----------------------------------------------------------------------------

pub(crate) fn process_dict_put(process: &ProcessTask, key: Atom, value: Term) -> Option<Term> {
  process.dict.insert(key, value)
}

pub(crate) fn process_dict_get(process: &ProcessTask, key: Atom) -> Option<Term> {
  process.dict.get(&key)
}

pub(crate) fn process_dict_delete(process: &ProcessTask, key: Atom) -> Option<Term> {
  process.dict.remove(&key)
}

pub(crate) fn process_dict_clear(process: &ProcessTask) -> Vec<(Atom, Term)> {
  process.dict.clear()
}

pub(crate) fn process_dict_pairs(process: &ProcessTask) -> Vec<(Atom, Term)> {
  process.dict.pairs()
}

pub(crate) fn process_dict_keys(process: &ProcessTask) -> Vec<Atom> {
  process.dict.keys()
}

pub(crate) fn process_dict_values(process: &ProcessTask) -> Vec<Term> {
  process.dict.values()
}
