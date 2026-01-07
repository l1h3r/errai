use std::sync::LazyLock;
use triomphe::Arc;

use crate::erts::ProcessSlot;
use crate::erts::ProcessTable;
use crate::erts::ProcessTask;
use crate::lang::Atom;
use crate::lang::RawPid;
use crate::lang::Term;

// The number of pre-allocated process states.
const CAP_REGISTERED_PROCS: usize = 1024;

// A table mapping internal process identifiers to process data.
static TABLE: LazyLock<ProcessTable<ProcessSlot>> = LazyLock::new(|| {
  ProcessTable::with_capacity(CAP_REGISTERED_PROCS)
});

// -----------------------------------------------------------------------------
// Common Utilities
// -----------------------------------------------------------------------------

pub(crate) fn translate_pid(pid: RawPid) -> Option<(u32, u32)> {
  if pid.into_bits() & RawPid::TAG_MASK == RawPid::TAG_DATA {
    Some(TABLE.translate_pid(pid))
  } else {
    None
  }
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
