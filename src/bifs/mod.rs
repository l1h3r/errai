use std::sync::LazyLock;

use crate::erts::ProcessSlot;
use crate::erts::ProcessTable;
use crate::lang::RawPid;

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
