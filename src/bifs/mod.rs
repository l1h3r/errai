//! Built-in functions implementing core process operations.
//!
//! This module contains the internal implementations of all process behavior,
//! including spawning, messaging, linking, monitoring, and name registration.
//! These functions are called "BIFs" (Built-In Functions) following Erlang
//! terminology.
//!
//! # Architecture
//!
//! BIFs form the layer between the public Process API and the underlying
//! process management infrastructure. They:
//!
//! - Validate arguments and enforce invariants
//! - Interact with global process and name tables
//! - Send signals between processes
//! - Manage process lifecycle
//!
//! # Global State
//!
//! Two global tables manage process state:
//!
//! - [`REGISTERED_PROCS`]: Maps PIDs to process data
//! - [`REGISTERED_NAMES`]: Maps registered names to PIDs
//!
//! These tables are initialized lazily and persist for the runtime's lifetime.
//!
//! # BEAM References
//!
//! Many functions reference corresponding BEAM (Erlang VM) implementations.
//! These links are maintained for cross-reference during development and to
//! aid in understanding expected behavior.
//!
//! OTP COMMIT: 11c6025cba47e24950cd4b4fc9f7e9e522388542
//!
//! Primary BEAM sources:
//!
//! - `erts/emulator/beam/bif.c`
//! - `erts/emulator/beam/erl_bif_info.c`
//! - `erts/emulator/beam/register.c`
//! - `erts/emulator/beam/erl_process_dict.c`

use hashbrown::HashMap;
use parking_lot::RwLock;
use std::sync::LazyLock;
use triomphe::Arc;

use crate::consts;
use crate::core::Atom;
use crate::core::InternalPid;
use crate::core::ProcTable;
use crate::proc::ProcData;
use crate::proc::ProcTask;

/// Global process table mapping PIDs to process data.
///
/// This table is the authoritative source for all process state. It uses
/// a lock-free implementation optimized for concurrent access.
static REGISTERED_PROCS: LazyLock<ProcTable<ProcData>> =
  LazyLock::new(|| ProcTable::with_capacity(consts::MAX_REGISTERED_PROCS));

/// Global name registry mapping registered names to PIDs.
///
/// Names provide stable addresses for processes. The table uses an RwLock
/// since name registration is infrequent compared to lookups.
static REGISTERED_NAMES: LazyLock<RwLock<HashMap<Atom, InternalPid>>> =
  LazyLock::new(|| RwLock::new(HashMap::with_capacity(consts::CAP_REGISTERED_NAMES)));

mod dictionary;
mod flags;
mod info;
mod link;
mod mailbox;
mod monitor;
mod name;
mod spawn;

pub(crate) use self::dictionary::proc_dict_clear;
pub(crate) use self::dictionary::proc_dict_delete;
pub(crate) use self::dictionary::proc_dict_get;
pub(crate) use self::dictionary::proc_dict_keys;
pub(crate) use self::dictionary::proc_dict_pairs;
pub(crate) use self::dictionary::proc_dict_put;
pub(crate) use self::dictionary::proc_dict_values;

pub(crate) use self::flags::proc_get_flags;
pub(crate) use self::flags::proc_set_flag;
pub(crate) use self::flags::proc_set_flags;

pub(crate) use self::info::proc_alive;
pub(crate) use self::info::proc_info;
pub(crate) use self::info::proc_list;

pub(crate) use self::link::proc_link;
pub(crate) use self::link::proc_unlink;

pub(crate) use self::mailbox::proc_receive;
pub(crate) use self::mailbox::proc_receive_any;
pub(crate) use self::mailbox::proc_receive_dyn;
pub(crate) use self::mailbox::proc_receive_exact;
pub(crate) use self::mailbox::proc_send;

pub(crate) use self::monitor::proc_demonitor;
pub(crate) use self::monitor::proc_monitor;

pub(crate) use self::name::proc_register;
pub(crate) use self::name::proc_registered;
pub(crate) use self::name::proc_unregister;
pub(crate) use self::name::proc_whereis;

pub(crate) use self::spawn::proc_exit;
pub(crate) use self::spawn::proc_remove;
pub(crate) use self::spawn::proc_spawn;
pub(crate) use self::spawn::proc_spawn_root;

// -----------------------------------------------------------------------------
// BIFs - Common Utils
// -----------------------------------------------------------------------------

/// Translates a PID into its (number, serial) components.
///
/// Returns `None` if the PID is invalid (incorrect tag bits).
///
/// Used for displaying PIDs in human-readable format.
pub(crate) fn translate_pid(pid: InternalPid) -> Option<(u32, u32)> {
  if pid.into_bits() & InternalPid::TAG_MASK == InternalPid::TAG_DATA {
    Some(REGISTERED_PROCS.translate_pid(pid))
  } else {
    None
  }
}

// -----------------------------------------------------------------------------
// Misc. Internal Utils
// -----------------------------------------------------------------------------

/// Looks up a process by PID in the global process table.
///
/// Returns `None` if the process doesn't exist or has been removed.
pub(crate) fn proc_find(pid: InternalPid) -> Option<Arc<ProcData>> {
  REGISTERED_PROCS.get(pid)
}

/// Helper for operations that may target the calling process or another process.
///
/// Optimizes the common case where the target is the calling process by
/// avoiding a table lookup. For other PIDs, performs a table lookup.
///
/// # Usage
///
/// ```ignore
/// proc_with(current_proc, target_pid, |proc| {
///   let Some(proc) = proc else {
///     // Target doesn't exist
///     return;
///   };
///   // Use proc...
/// });
/// ```
pub(crate) fn proc_with<F, R>(this: &ProcTask, pid: InternalPid, f: F) -> R
where
  F: FnOnce(Option<&ProcData>) -> R,
{
  let hold: Arc<ProcData>;
  let data: &ProcData;

  if this.readonly.mpid == pid {
    data = this;
  } else {
    let Some(context) = proc_find(pid) else {
      return f(None);
    };

    hold = context;
    data = &hold;
  }

  f(Some(data))
}
