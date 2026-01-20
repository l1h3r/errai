// -----------------------------------------------------------------------------
// Process Link
//
// BEAM Reference:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_monitor_link.h
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_monitor_link.c
// -----------------------------------------------------------------------------

use hashbrown::hash_map::Entry;
use std::num::NonZeroU64;

use crate::bifs::proc_find;
use crate::core::Exit;
use crate::core::InternalPid;
use crate::core::ProcessId;
use crate::proc::ProcLink;
use crate::proc::ProcTask;

/// Creates a bidirectional link between the calling process and the target.
///
/// # Link Protocol
///
/// 1. Check if link already exists (no-op if active, enable if disabled)
/// 2. Add link to caller's link table
/// 3. Send LINK signal to target (or LINK_EXIT if target is dead)
///
/// # Special Cases
///
/// - Self-linking is ignored (no-op)
/// - Linking to dead process sends immediate LINK_EXIT
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L134>
pub(crate) fn proc_link<P>(this: &ProcTask, pid: P)
where
  P: ProcessId,
{
  if P::DISTRIBUTED {
    todo!("Handle External PID Link: {pid}")
  }

  let pid: InternalPid = pid.into_internal();

  if this.readonly.mpid == pid {
    return; // Ignore, we can't link to ourselves
  }

  match this.internal().links.entry(pid) {
    Entry::Occupied(mut entry) => {
      if entry.get().is_enabled() {
        return; // Ignore, we dont need to update an active link
      }

      entry.get_mut().enable();
    }
    Entry::Vacant(entry) => {
      entry.insert(ProcLink::new());
    }
  }

  if let Some(proc) = proc_find(pid) {
    proc.readonly.send_link(this.readonly.mpid);
  } else {
    this.readonly.send_link_exit(pid, Exit::NOPROC);
  }
}

/// Removes a bidirectional link between the calling process and the target.
///
/// # Unlink Protocol
///
/// 1. Check if link exists (no-op if not)
/// 2. Disable link with unique ID (prevents exit signal races)
/// 3. Send UNLINK signal with ID to target
/// 4. Wait for UNLINK_ACK to complete removal
///
/// # Special Cases
///
/// - No-op if no link exists
/// - Immediate removal if target is already dead
/// - No-op if link is already disabled
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L1212>
pub(crate) fn proc_unlink<P>(this: &ProcTask, pid: P)
where
  P: ProcessId,
{
  if P::DISTRIBUTED {
    todo!("Handle External PID Unlink: {pid}")
  }

  let pid: InternalPid = pid.into_internal();

  match this.internal().links.entry(pid) {
    Entry::Occupied(mut entry) => {
      if entry.get().is_disabled() {
        return; // Ignore, we've been here before
      }

      if let Some(proc) = proc_find(pid) {
        let unlink: NonZeroU64 = this.readonly.next_puid();

        proc.readonly.send_unlink(this.readonly.mpid, unlink);

        entry.get_mut().disable(unlink);
      } else {
        entry.remove();
      }
    }
    Entry::Vacant(_) => {
      return; // Ignore, we're not linked to PID
    }
  }
}
