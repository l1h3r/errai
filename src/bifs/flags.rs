// -----------------------------------------------------------------------------
// Process Flags
//
// BEAM Reference:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process.h#L1632
// -----------------------------------------------------------------------------

use crate::erts::ProcessFlags;
use crate::proc::ProcTask;

/// Returns the process flags of the calling process.
///
/// BEAM Builtin: N/A
pub(crate) fn proc_get_flags(this: &ProcTask) -> ProcessFlags {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().flags
}

/// Sets all process flags for the calling process.
///
/// Replaces the entire flag set with the provided value.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2013>
pub(crate) fn proc_set_flags(this: &ProcTask, flags: ProcessFlags) {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().flags = flags;
}

/// Sets a single process flag to the given value.
///
/// Other flags remain unchanged.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2013>
pub(crate) fn proc_set_flag(this: &ProcTask, flag: ProcessFlags, value: bool) {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().flags.set(flag, value);
}
