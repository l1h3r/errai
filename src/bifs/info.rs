// -----------------------------------------------------------------------------
// Process Info
//
// BEAM Reference:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_bif_info.c
// -----------------------------------------------------------------------------

use crate::core::InternalPid;
use crate::erts::ProcessInfo;
use crate::proc::ProcTask;

use super::REGISTERED_PROCS;

/// Returns a list of all currently existing process identifiers.
///
/// Includes exiting processes (not yet fully terminated). The returned list
/// is a snapshot and may be stale immediately after returning.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L4078>
pub(crate) fn proc_list() -> Vec<InternalPid> {
  let capacity: usize = REGISTERED_PROCS.len();
  let mut data: Vec<InternalPid> = Vec::with_capacity(capacity);

  for pid in REGISTERED_PROCS.keys() {
    data.push(pid);
  }

  data
}

/// Checks if a process is alive.
///
/// Returns `true` if the process exists and has not started exiting.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_bif_info.c#L3990>
pub(crate) fn proc_alive(_this: &ProcTask, _pid: InternalPid) -> bool {
  todo!("Handle Process Alive")
}

/// Returns detailed information about a process.
///
/// Returns `None` if the process is not alive.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_bif_info.c#L1558>
pub(crate) fn proc_info(_this: &ProcTask, _pid: InternalPid) -> Option<ProcessInfo> {
  todo!("Handle Process Info")
}
