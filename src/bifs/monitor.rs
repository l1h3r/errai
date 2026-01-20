// -----------------------------------------------------------------------------
// Process Monitor
//
// BEAM Reference:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_monitor_link.h
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_monitor_link.c
// -----------------------------------------------------------------------------

use hashbrown::hash_map::Entry;

use crate::bifs::proc_find;
use crate::bifs::proc_whereis;
use crate::core::Exit;
use crate::core::ExternalDest;
use crate::core::InternalPid;
use crate::core::MonitorRef;
use crate::proc::ProcMonitor;
use crate::proc::ProcTask;
use crate::raise;

/// Establishes a unidirectional monitor on the target.
///
/// # Monitor Protocol
///
/// 1. Generate unique monitor reference
/// 2. Add monitor to caller's monitor_send table
/// 3. Send MONITOR signal to target (or immediate DOWN if dead)
///
/// # Name Resolution
///
/// For name-based destinations, the name is resolved immediately. If the
/// name is unregistered, an immediate DOWN message is sent.
///
/// # Special Cases
///
/// - Self-monitoring returns a reference but is otherwise ignored
/// - Monitoring dead process sends immediate DOWN
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L753>
pub(crate) fn proc_monitor(this: &ProcTask, item: ExternalDest) -> MonitorRef {
  match item {
    ExternalDest::InternalProc(pid) => {
      let mref: MonitorRef = MonitorRef::new();

      if this.readonly.mpid == pid {
        return mref; // Ignore, we can't monitor ourselves
      }

      match this.internal().monitor_send.entry(mref) {
        Entry::Occupied(_) => {
          raise!(Error, SysInv, "duplicate monitor");
        }
        Entry::Vacant(entry) => {
          entry.insert(ProcMonitor::new(this.readonly.mpid, item));
        }
      }

      if let Some(proc) = proc_find(pid) {
        proc.readonly.send_monitor(this.readonly.mpid, mref, pid);
      } else {
        this.readonly.send_monitor_down(pid, mref, Exit::NOPROC);
      }

      mref
    }
    ExternalDest::InternalName(name) => {
      if let Some(pid) = proc_whereis(name) {
        proc_monitor(this, ExternalDest::InternalProc(pid))
      } else {
        let mref: MonitorRef = MonitorRef::new();

        match this.internal().monitor_send.entry(mref) {
          Entry::Occupied(_) => {
            raise!(Error, SysInv, "duplicate monitor");
          }
          Entry::Vacant(entry) => {
            entry.insert(ProcMonitor::new(this.readonly.mpid, item));
          }
        }

        this
          .readonly
          .send_monitor_down(InternalPid::UNDEFINED, mref, Exit::NOPROC);

        mref
      }
    }
    ExternalDest::ExternalProc(pid) => {
      todo!("Handle External PID Monitor - {pid}")
    }
    ExternalDest::ExternalName(name, node) => {
      todo!("Handle External Name Monitor - {name} @ {node}")
    }
  }
}

/// Removes a monitor on the target process.
///
/// # Demonitor Protocol
///
/// 1. Remove monitor from caller's monitor_send table
/// 2. Send DEMONITOR signal to target (if still alive)
///
/// # Special Cases
///
/// - No-op if monitor reference doesn't exist
/// - No-op if target is already dead
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L433>
pub(crate) fn proc_demonitor(this: &ProcTask, mref: MonitorRef) {
  match this.internal().monitor_send.entry(mref) {
    Entry::Occupied(entry) => {
      let state: ProcMonitor = entry.remove();

      match state.target() {
        ExternalDest::InternalProc(pid) => {
          if let Some(proc) = proc_find(pid) {
            proc.readonly.send_demonitor(this.readonly.mpid, mref);
          } else {
            return; // Ignore, dead PID
          }
        }
        ExternalDest::InternalName(name) => {
          if let Some(pid) = proc_whereis(name) {
            if let Some(proc) = proc_find(pid) {
              proc.readonly.send_demonitor(this.readonly.mpid, mref);
            } else {
              return; // Ignore, dead PID
            }
          } else {
            return; // Ignore, dead PID
          }
        }
        ExternalDest::ExternalProc(pid) => {
          todo!("Handle External PID demonitor - {pid}")
        }
        ExternalDest::ExternalName(name, node) => {
          todo!("Handle External Name demonitor - {name} @ {node}")
        }
      }
    }
    Entry::Vacant(_) => {
      return; // Ignore, we're not monitoring this ref
    }
  }
}
