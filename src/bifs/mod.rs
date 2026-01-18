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
use hashbrown::hash_map::Entry;
use parking_lot::Mutex;
use parking_lot::MutexGuard;
use parking_lot::RwLock;
use parking_lot::RwLockWriteGuard;
use std::mem::MaybeUninit;
use std::num::NonZeroU64;
use std::panic::AssertUnwindSafe;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::task;
use tokio::task::futures::TaskLocalFuture;
use tracing::Span;
use tracing::span;
use triomphe::Arc;

use crate::consts;
use crate::core::Atom;
use crate::core::Exit;
use crate::core::ExternalDest;
use crate::core::InternalDest;
use crate::core::InternalPid;
use crate::core::MonitorRef;
use crate::core::ProcTable;
use crate::core::ProcessId;
use crate::core::Term;
use crate::core::TimerRef;
use crate::erts::DynMessage;
use crate::erts::Message;
use crate::erts::Process;
use crate::erts::ProcessFlags;
use crate::erts::ProcessInfo;
use crate::erts::Signal;
use crate::erts::SignalRecv;
use crate::erts::SpawnConfig;
use crate::erts::SpawnHandle;
use crate::proc::ProcData;
use crate::proc::ProcExternal;
use crate::proc::ProcInternal;
use crate::proc::ProcLink;
use crate::proc::ProcMail;
use crate::proc::ProcMonitor;
use crate::proc::ProcReadOnly;
use crate::proc::ProcRecv;
use crate::proc::ProcSend;
use crate::proc::ProcTask;
use crate::proc::unbounded_channel;
use crate::raise;
use crate::utils::CatchUnwind;

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

// -----------------------------------------------------------------------------
// General
//
// BEAM Builtin:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_bif_info.c
// -----------------------------------------------------------------------------

/// Returns a list of all currently existing process identifiers.
///
/// Includes exiting processes (not yet fully terminated). The returned list
/// is a snapshot and may be stale immediately after returning.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L4078>
pub(crate) fn process_list() -> Vec<InternalPid> {
  let capacity: usize = REGISTERED_PROCS.len();
  let mut data: Vec<InternalPid> = Vec::with_capacity(capacity);

  for pid in REGISTERED_PROCS.keys() {
    data.push(pid);
  }

  data
}

/// Sends an exit signal to the target process.
///
/// The signal is sent unconditionally. The target's handling depends on:
///
/// - Exit reason (normal/kill/custom)
/// - Process flags (trap_exit)
/// - Whether the process is linked to the sender
///
/// Never fails for non-existent PIDs (signals are silently dropped).
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L1710>
//
// TODO: Support terminating via reference
pub(crate) fn process_exit<P>(this: &ProcTask, pid: P, exit: Exit)
where
  P: ProcessId,
{
  if P::DISTRIBUTED {
    todo!("Handle External PID Exit: {pid}")
  }

  let pid: InternalPid = pid.into_internal();

  process_with(this, pid, |proc| {
    let Some(proc) = proc else {
      return; // Never fail when sending to a non-existent PID.
    };

    proc.readonly.send_exit(this.readonly.mpid, exit);
  });
}

/// Checks if a process is alive.
///
/// Returns `true` if the process exists and has not started exiting.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_bif_info.c#L3990>
pub(crate) fn process_alive(_this: &ProcTask, _pid: InternalPid) -> bool {
  todo!("Handle Process Alive")
}

/// Returns the process flags of the calling process.
///
/// BEAM Builtin: N/A
pub(crate) fn process_get_flags(this: &ProcTask) -> ProcessFlags {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().flags
}

/// Sets all process flags for the calling process.
///
/// Replaces the entire flag set with the provided value.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2013>
pub(crate) fn process_set_flags(this: &ProcTask, flags: ProcessFlags) {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().flags = flags;
}

/// Sets a single process flag to the given value.
///
/// Other flags remain unchanged.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2013>
pub(crate) fn process_set_flag(this: &ProcTask, flag: ProcessFlags, value: bool) {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().flags.set(flag, value);
}

/// Returns detailed information about a process.
///
/// Returns `None` if the process is not alive.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_bif_info.c#L1558>
pub(crate) fn process_info(_this: &ProcTask, _pid: InternalPid) -> Option<ProcessInfo> {
  todo!("Handle Process Info")
}

// -----------------------------------------------------------------------------
// Spawning & Messaging
// -----------------------------------------------------------------------------

/// Spawns a new process with the given configuration.
///
/// The spawned process:
///
/// 1. Executes the provided future
/// 2. Has its own signal queue and state
/// 3. May be linked/monitored based on `opts`
///
/// Returns a handle containing the PID and optional monitor reference.
pub(crate) fn process_spawn<F>(this: &ProcTask, opts: SpawnConfig, future: F) -> SpawnHandle
where
  F: Future<Output = ()> + Send + 'static,
{
  process_spawn_internal(this.into(), future, opts)
}

/// Spawns the root supervisor process.
///
/// The root process has no parent and is used to supervise the entire
/// application. It must not be spawned with monitoring enabled.
pub(crate) fn process_spawn_root<F>(future: F) -> InternalPid
where
  F: Future<Output = ()> + Send + 'static,
{
  match process_spawn_internal(None, future, SpawnConfig::new()) {
    SpawnHandle::Process(pid) => pid,
    SpawnHandle::Monitor(_, _) => raise!(Error, SysInv, "root process monitor"),
  }
}

/// Sends a message to the given destination.
///
/// # Destination Handling
///
/// - **InternalProc**: Sends directly to the PID
/// - **InternalName**: Resolves name, then sends (raises if unregistered)
/// - **ExternalProc/ExternalName**: Currently unimplemented
///
/// # Error Handling
///
/// - Silently drops messages to dead PIDs
/// - Raises exception for unregistered names
pub(crate) fn process_send(this: &ProcTask, dest: ExternalDest, term: Term) {
  match dest {
    ExternalDest::InternalProc(pid) => {
      process_with(this, pid, |proc| {
        let Some(proc) = proc else {
          return; // Never fail when sending to a non-existent PID.
        };

        proc.readonly.send_message(this.readonly.mpid, term);
      });
    }
    ExternalDest::InternalName(name) => {
      let Some(pid) = process_whereis(name) else {
        raise!(Error, BadArg, "unregistered name");
      };

      process_with(this, pid, |proc| {
        let Some(proc) = proc else {
          raise!(Error, BadArg, "unregistered name");
        };

        proc.readonly.send_message(this.readonly.mpid, term);
      });
    }
    ExternalDest::ExternalProc(pid) => {
      todo!("Handle External PID Send - {pid}")
    }
    ExternalDest::ExternalName(name, node) => {
      todo!("Handle External Name Send - {name} @ {node}")
    }
  }
}

/// Receives a message matching type `T` from the mailbox.
///
/// Wraps the message in a [`Message`] envelope that may also contain
/// EXIT or DOWN signals.
pub(crate) async fn process_receive<T>(pid: InternalPid) -> Message<Box<T>>
where
  T: 'static,
{
  ProcMail::receive::<T>(pid).await
}

/// Receives an exact message of type `T` from the mailbox.
///
/// Returns the unwrapped value without the [`Message`] envelope.
pub(crate) async fn process_receive_exact<T>(pid: InternalPid) -> Box<T>
where
  T: 'static,
{
  ProcMail::receive_exact::<T>(pid).await
}

/// Receives any message from the mailbox without filtering.
pub(crate) async fn process_receive_any(pid: InternalPid) -> DynMessage {
  ProcMail::receive_any(pid).await
}

/// Receives a message matching a custom filter function.
///
/// The filter is called for each message until one matches.
pub(crate) async fn process_receive_dyn<F>(pid: InternalPid, filter: F) -> DynMessage
where
  F: Fn(&DynMessage) -> bool,
{
  ProcMail::receive_dyn(pid, filter).await
}

// -----------------------------------------------------------------------------
// Links, Monitors, Aliases
// -----------------------------------------------------------------------------

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
pub(crate) fn process_link<P>(this: &ProcTask, pid: P)
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

  match this.internal.lock().links.entry(pid) {
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

  if let Some(proc) = process_find(pid) {
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
pub(crate) fn process_unlink<P>(this: &ProcTask, pid: P)
where
  P: ProcessId,
{
  if P::DISTRIBUTED {
    todo!("Handle External PID Unlink: {pid}")
  }

  let pid: InternalPid = pid.into_internal();

  match this.internal.lock().links.entry(pid) {
    Entry::Occupied(mut entry) => {
      if entry.get().is_disabled() {
        return; // Ignore, we've been here before
      }

      if let Some(proc) = process_find(pid) {
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
pub(crate) fn process_monitor(this: &ProcTask, item: ExternalDest) -> MonitorRef {
  match item {
    ExternalDest::InternalProc(pid) => {
      let mref: MonitorRef = MonitorRef::new();

      if this.readonly.mpid == pid {
        return mref; // Ignore, we can't monitor ourselves
      }

      match this.internal.lock().monitor_send.entry(mref) {
        Entry::Occupied(_) => {
          raise!(Error, SysInv, "duplicate monitor");
        }
        Entry::Vacant(entry) => {
          entry.insert(ProcMonitor::new(this.readonly.mpid, item));
        }
      }

      if let Some(proc) = process_find(pid) {
        proc.readonly.send_monitor(this.readonly.mpid, mref, pid);
      } else {
        this.readonly.send_monitor_down(pid, mref, Exit::NOPROC);
      }

      mref
    }
    ExternalDest::InternalName(name) => {
      if let Some(pid) = process_whereis(name) {
        process_monitor(this, ExternalDest::InternalProc(pid))
      } else {
        let mref: MonitorRef = MonitorRef::new();

        match this.internal.lock().monitor_send.entry(mref) {
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
pub(crate) fn process_demonitor(this: &ProcTask, mref: MonitorRef) {
  match this.internal.lock().monitor_send.entry(mref) {
    Entry::Occupied(entry) => {
      let state: ProcMonitor = entry.remove();

      match state.target() {
        ExternalDest::InternalProc(pid) => {
          if let Some(proc) = process_find(pid) {
            proc.readonly.send_demonitor(this.readonly.mpid, mref);
          } else {
            return; // Ignore, dead PID
          }
        }
        ExternalDest::InternalName(name) => {
          if let Some(pid) = process_whereis(name) {
            if let Some(proc) = process_find(pid) {
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

// -----------------------------------------------------------------------------
// Timers
// -----------------------------------------------------------------------------

pub(crate) fn process_timer_create<T>(
  this: &ProcTask,
  dest: InternalDest,
  term: T,
  time: Duration,
) -> TimerRef
where
  T: Send + 'static,
{
  todo!("TODO: Handle Process Timer Create")
}

pub(crate) fn process_timer_stop(timer: TimerRef, non_blocking: bool) -> Option<Duration> {
  todo!("TODO: Handle Process Timer Stop")
}

pub(crate) fn process_timer_read(timer: TimerRef, non_blocking: bool) -> Option<Duration> {
  todo!("TODO: Handle Process Timer Read")
}

// -----------------------------------------------------------------------------
// Local Name Registration
//
// BEAM Builtin:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.h
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.c
// -----------------------------------------------------------------------------

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
pub(crate) fn process_register(this: &ProcTask, pid: InternalPid, name: Atom) {
  if name == Atom::UNDEFINED {
    raise!(Error, BadArg, "reserved name");
  }

  let mut name_guard: RwLockWriteGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.write();

  let Entry::Vacant(name_entry) = name_guard.entry(name) else {
    raise!(Error, BadArg, "registered name");
  };

  process_with(this, pid, |proc| {
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
pub(crate) fn process_unregister(this: &ProcTask, name: Atom) {
  let mut name_guard: RwLockWriteGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.write();

  let Entry::Occupied(name_entry) = name_guard.entry(name) else {
    raise!(Error, BadArg, "unregistered name");
  };

  process_with(this, *name_entry.get(), |proc| {
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
pub(crate) fn process_whereis(name: Atom) -> Option<InternalPid> {
  REGISTERED_NAMES.read().get(&name).copied()
}

/// Returns all currently registered names.
///
/// Returns a snapshot of registered names. The list may be stale immediately
/// after returning.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.c#L576>
pub(crate) fn process_registered() -> Vec<Atom> {
  Vec::from_iter(REGISTERED_NAMES.read().keys().copied())
}

// -----------------------------------------------------------------------------
// Process Dictionary
//
// BEAM Builtin:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.h
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c
// -----------------------------------------------------------------------------

/// Stores a key-value pair in the process dictionary.
///
/// Returns the previous value if the key was already set.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L353>
pub(crate) fn process_dict_put(this: &ProcTask, key: Atom, value: Term) -> Option<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.insert(key, value)
}

/// Retrieves a value from the process dictionary.
///
/// Returns `None` if the key is not set.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L324>
pub(crate) fn process_dict_get(this: &ProcTask, key: Atom) -> Option<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.get(&key)
}

/// Deletes a key from the process dictionary.
///
/// Returns the previous value if the key was set.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L373>
pub(crate) fn process_dict_delete(this: &ProcTask, key: Atom) -> Option<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.remove(&key)
}

/// Clears the entire process dictionary.
///
/// Returns all key-value pairs that were stored.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L363>
pub(crate) fn process_dict_clear(this: &ProcTask) -> Vec<(Atom, Term)> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.clear()
}

/// Returns all key-value pairs in the process dictionary.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L315>
pub(crate) fn process_dict_pairs(this: &ProcTask) -> Vec<(Atom, Term)> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.pairs()
}

/// Returns all keys in the process dictionary.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L333>
pub(crate) fn process_dict_keys(this: &ProcTask) -> Vec<Atom> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.keys()
}

/// Returns all values in the process dictionary.
///
/// BEAM Builtin: N/A
pub(crate) fn process_dict_values(this: &ProcTask) -> Vec<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.values()
}

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
// BIFs - Spawn Utils
// -----------------------------------------------------------------------------

/// Removes a process from all global state during termination.
///
/// # Cleanup Protocol
///
/// 1. Lock process state (internal + external)
/// 2. Extract exit reason (or default to "panic")
/// 3. Unregister name if present
/// 4. Send LINK_EXIT to all linked processes
/// 5. Send MONITOR_DOWN to all monitoring processes
/// 6. Remove from process table
/// 7. Unlock and complete
///
/// # Invocation
///
/// Called automatically by [`ProcTask::drop()`] when a process task completes.
pub(crate) fn process_delete(this: &mut ProcTask) {
  let span: Span = tracing::trace_span!("Proc Delete", pid = %this.readonly.mpid);
  let span_guard: span::Entered<'_> = span.enter();

  tracing::trace!("(1) - Lock");

  let mut internal: MutexGuard<'_, ProcInternal> = this.internal.lock();
  let mut external: RwLockWriteGuard<'_, ProcExternal> = this.external.write();

  let name: Option<Atom> = external.name.clone();

  let exit: Exit = match external.exit.take() {
    Some(data) => data,
    None => Exit::Atom(Atom::new("panic")),
  };

  drop(external);

  tracing::trace!("(2) - Unregister Name");

  if let Some(name) = name {
    if let None = REGISTERED_NAMES.write().remove(&name) {
      tracing::error!(%name, "dangling name");
    }
  }

  tracing::trace!("(3) - Send Link `EXIT`");

  for (pid, state) in internal.links.drain() {
    if state.is_disabled() {
      continue;
    }

    if let Some(proc) = process_find(pid) {
      proc
        .readonly
        .send_link_exit(this.readonly.mpid, exit.clone());
    }
  }

  tracing::trace!("(4) - Send Monitor `DOWN`");

  for (mref, state) in internal.monitor_recv.drain() {
    if let Some(proc) = process_find(state.origin()) {
      proc
        .readonly
        .send_monitor_down(state.origin(), mref, exit.clone());
    }
  }

  tracing::trace!("(5) - State");

  if let None = REGISTERED_PROCS.remove(this.readonly.mpid) {
    tracing::error!("dangling state");
  };

  tracing::trace!("(6) - Unlock");

  drop(internal);

  drop(span_guard);
}

/// Creates a new process with the given configuration.
///
/// # Initialization
///
/// 1. Create signal queue (sender + receiver)
/// 2. Initialize process state (readonly, internal, external)
/// 3. Apply spawn options (flags)
/// 4. Insert into process table (atomically assigns PID)
///
/// Returns the process data Arc and signal receiver.
///
/// # Panics
///
/// Raises exception if the process table is full.
fn process_create(parent: Option<&ProcTask>, options: SpawnConfig) -> (Arc<ProcData>, ProcRecv) {
  let parent_pid: Option<InternalPid> = parent.map(|process| process.readonly.mpid);
  let leader_pid: Option<InternalPid> = parent.map(|process| process.internal.lock().group_leader);

  let (sig_send, sig_recv): (ProcSend, ProcRecv) = unbounded_channel();

  let mut readonly: ProcReadOnly = ProcReadOnly::new(sig_send, parent_pid);
  let mut internal: ProcInternal = ProcInternal::new();

  let external: ProcExternal = ProcExternal::new();

  internal
    .flags
    .set(ProcessFlags::TRAP_EXIT, options.trap_exit);

  internal
    .flags
    .set(ProcessFlags::ASYNC_DIST, options.async_dist);

  let initialize = |uninit: &mut MaybeUninit<ProcData>, pid: InternalPid| {
    readonly.mpid = pid;
    internal.group_leader = leader_pid.unwrap_or(readonly.mpid);

    let proc_state: *mut ProcData = uninit.as_mut_ptr();

    // SAFETY: We're initializing each field exactly once in the correct order.
    //         The ProcData fields are initialized via raw pointer writes because
    //         the ProcTable insert callback works with MaybeUninit.
    unsafe {
      (&raw mut (*proc_state).readonly).write(readonly);
      (&raw mut (*proc_state).internal).write(Mutex::new(internal));
      (&raw mut (*proc_state).external).write(RwLock::new(external));
    }
  };

  let Ok(proc) = REGISTERED_PROCS.insert(initialize) else {
    raise!(Error, SysCap, "too many processes");
  };

  (proc, sig_recv)
}

/// Internal spawn implementation shared by all spawn functions.
///
/// # Spawn Sequence
///
/// 1. **Create Process**: Allocate state and assign PID
/// 2. **Create Task**: Wrap future in task-local context
/// 3. **Initialize Link**: Establish link if requested
/// 4. **Initialize Monitor**: Establish monitor if requested
/// 5. **Spawn Task**: Schedule process future on Tokio
/// 6. **Return Handle**: Return PID (+ monitor ref if monitored)
///
/// # Task Loop
///
/// The spawned task runs a select loop that:
/// - Processes incoming signals (exit, link, monitor, message)
/// - Executes the user-provided future
/// - Catches panics and converts them to exit reasons
/// - Stores exit reason on termination
///
/// # Panic Handling
///
/// User code panics are caught and converted to `Exit::Term` containing
/// the panic payload. This prevents panics from escaping the process
/// boundary.
fn process_spawn_internal<F>(
  parent: Option<&ProcTask>,
  future: F,
  options: SpawnConfig,
) -> SpawnHandle
where
  F: Future<Output = ()> + Send + 'static,
{
  // ---------------------------------------------------------------------------
  // 1. Create Process
  // ---------------------------------------------------------------------------

  let (proc, sig_recv): (Arc<ProcData>, ProcRecv) = process_create(parent, options);

  // ---------------------------------------------------------------------------
  // 2. Create Process Task
  // ---------------------------------------------------------------------------

  let context: ProcTask = ProcTask {
    inner: Arc::clone(&proc),
  };

  let local: TaskLocalFuture<ProcTask, _> = Process::scope(context, async move {
    let mut queue: ProcRecv = sig_recv;
    let safe_task: CatchUnwind<AssertUnwindSafe<F>> = CatchUnwind::new(AssertUnwindSafe(future));

    tokio::pin!(safe_task);

    // Run signal processing + user future until termination
    let exit: Exit = 'run: loop {
      // Process all pending signals before waiting
      while let Ok(signal) = queue.try_recv() {
        if let Some(exit) = Process::with(|this| process_handle_signal(this, signal)) {
          break 'run exit;
        }
      }

      tokio::select! {
        biased;
        // Prioritize signals over user code
        Some(signal) = queue.recv() => {
          if let Some(exit) = Process::with(|this| process_handle_signal(this, signal)) {
            break 'run exit;
          }
        }
        // Execute user future, catching panics
        result = &mut safe_task => match result {
          Ok(()) => break 'run Exit::NORMAL,
          Err(term) => break 'run Exit::Term(Term::new_error(term)),
        }
      }
    };

    // Store exit reason for cleanup
    Process::with(|this| {
      if let Err(_) = this.external.read().exit.set(exit) {
        raise!(Error, SysInv, "duplicate termination reason");
      }
    });
  });

  // ---------------------------------------------------------------------------
  // 3. Initialize Link
  // ---------------------------------------------------------------------------

  if options.link {
    let Some(root) = parent else {
      raise!(Error, SysInv, "link without parent");
    };

    process_link(root, proc.readonly.mpid);
  }

  // ---------------------------------------------------------------------------
  // 4. Initialize Monitor
  // ---------------------------------------------------------------------------

  let mut handle: SpawnHandle = SpawnHandle::Process(proc.readonly.mpid);

  if options.monitor {
    let Some(root) = parent else {
      raise!(Error, SysInv, "monitor without parent");
    };

    let mtarget: ExternalDest = ExternalDest::InternalProc(proc.readonly.mpid);
    let monitor: MonitorRef = process_monitor(root, mtarget);
    let mhandle: SpawnHandle = SpawnHandle::Monitor(proc.readonly.mpid, monitor);

    handle = mhandle;
  }

  // ---------------------------------------------------------------------------
  // 5. Spawn Task
  // ---------------------------------------------------------------------------

  if let Some(root) = parent {
    tracing::trace!(pid = %proc.readonly.mpid, from = %root.readonly.mpid, "Proc Spawn");
  } else {
    tracing::trace!(pid = %proc.readonly.mpid, "Proc Spawn");
  }

  let Ok(()) = proc.readonly.task.set(task::spawn(local)) else {
    raise!(Error, SysInv, "duplicate task");
  };

  // ---------------------------------------------------------------------------
  // 6. Return
  // ---------------------------------------------------------------------------

  handle
}

// -----------------------------------------------------------------------------
// Misc. Internal Utils
// -----------------------------------------------------------------------------

/// Looks up a process by PID in the global process table.
///
/// Returns `None` if the process doesn't exist or has been removed.
pub(crate) fn process_find(pid: InternalPid) -> Option<Arc<ProcData>> {
  REGISTERED_PROCS.get(pid)
}

/// Processes a signal in the context of the receiving process.
///
/// Returns `Some(Exit)` if the signal should terminate the process.
fn process_handle_signal(this: &ProcTask, signal: Signal) -> Option<Exit> {
  signal.recv(&this.readonly, &mut this.internal.lock())
}

/// Helper for operations that may target the calling process or another process.
///
/// Optimizes the common case where the target is the calling process by
/// avoiding a table lookup. For other PIDs, performs a table lookup.
///
/// # Usage
///
/// ```ignore
/// process_with(current_proc, target_pid, |proc| {
///   let Some(proc) = proc else {
///     // Target doesn't exist
///     return;
///   };
///   // Use proc...
/// });
/// ```
fn process_with<F, R>(this: &ProcTask, pid: InternalPid, f: F) -> R
where
  F: FnOnce(Option<&ProcData>) -> R,
{
  let hold: Arc<ProcData>;
  let data: &ProcData;

  if this.readonly.mpid == pid {
    data = this;
  } else {
    let Some(context) = process_find(pid) else {
      return f(None);
    };

    hold = context;
    data = &hold;
  }

  f(Some(data))
}
