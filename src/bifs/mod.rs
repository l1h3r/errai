// OTP COMMIT: 11c6025cba47e24950cd4b4fc9f7e9e522388542
use hashbrown::HashMap;
use hashbrown::hash_map::Entry;
use parking_lot::Mutex;
use parking_lot::MutexGuard;
use parking_lot::RwLock;
use parking_lot::RwLockWriteGuard;
use std::mem::MaybeUninit;
use std::num::NonZeroU64;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::LazyLock;
use tokio::task;
use tokio::task::futures::TaskLocalFuture;
use triomphe::Arc;

use crate::core::CatchUnwind;
use crate::core::ProcData;
use crate::core::ProcExternal;
use crate::core::ProcInternal;
use crate::core::ProcLink;
use crate::core::ProcMail;
use crate::core::ProcReadOnly;
use crate::core::ProcRecv;
use crate::core::ProcSend;
use crate::core::ProcTask;
use crate::core::ProcessFlags;
use crate::core::ProcessInfo;
use crate::core::raise;
use crate::core::unbounded_channel;
use crate::erts::DynMessage;
use crate::erts::ExitMessage;
use crate::erts::Message;
use crate::erts::ProcTable;
use crate::erts::Process;
use crate::erts::Runtime;
use crate::erts::Signal;
use crate::erts::SpawnConfig;
use crate::erts::SpawnHandle;
use crate::lang::Atom;
use crate::lang::Exit;
use crate::lang::ExternalDest;
use crate::lang::InternalDest;
use crate::lang::InternalPid;
use crate::lang::InternalRef;
use crate::lang::MonitorRef;
use crate::lang::ProcessId;
use crate::lang::Term;

// A table mapping internal process identifiers to process data.
static REGISTERED_PROCS: LazyLock<ProcTable<ProcData>> =
  LazyLock::new(|| ProcTable::with_capacity(Runtime::CAP_REGISTERED_PROCS));

// A table mapping registered names to internal process identifiers.
static REGISTERED_NAMES: LazyLock<RwLock<HashMap<Atom, InternalPid>>> =
  LazyLock::new(|| RwLock::new(HashMap::with_capacity(Runtime::CAP_REGISTERED_NAMES)));

// -----------------------------------------------------------------------------
// General
//
// BEAM Builtin:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_bif_info.c
// -----------------------------------------------------------------------------

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L4078
pub(crate) fn process_list() -> Vec<InternalPid> {
  let capacity: usize = REGISTERED_PROCS.len();
  let mut data: Vec<InternalPid> = Vec::with_capacity(capacity);

  for pid in REGISTERED_PROCS.keys() {
    data.push(pid);
  }

  data
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L1710
//
// TODO: Support terminating via reference
pub(crate) fn process_exit<P>(this: &ProcTask, pid: P, reason: Exit)
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

    proc.readonly.send.send(Signal::Exit {
      from: this.readonly.mpid,
      reason,
      linked: false,
    });
  });
}

// BEAM Builtin: N/A
pub(crate) fn process_get_flags(this: &ProcTask) -> ProcessFlags {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().flags
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2013
pub(crate) fn process_set_flags(this: &ProcTask, flags: ProcessFlags) {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().flags = flags;
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2013
pub(crate) fn process_set_flag(this: &ProcTask, flag: ProcessFlags, value: bool) {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().flags.set(flag, value);
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_bif_info.c#L1558
pub(crate) fn process_info(_this: &ProcTask, _pid: InternalPid) -> Option<ProcessInfo> {
  todo!()
}

// -----------------------------------------------------------------------------
// Spawning & Messaging
// -----------------------------------------------------------------------------

pub(crate) fn process_spawn<F>(this: &ProcTask, opts: SpawnConfig, future: F) -> SpawnHandle
where
  F: Future<Output = ()> + Send + 'static,
{
  process_spawn_internal(this.into(), future, opts)
}

pub(crate) fn process_spawn_root<F>(future: F) -> InternalPid
where
  F: Future<Output = ()> + Send + 'static,
{
  match process_spawn_internal(None, future, SpawnConfig::new()) {
    SpawnHandle::Process(pid) => pid,
    SpawnHandle::Monitor(_, _) => raise!(Error, SysInv, "root process monitor"),
  }
}

pub(crate) fn process_send(this: &ProcTask, dest: ExternalDest, term: Term) {
  match dest {
    ExternalDest::InternalProc(pid) => {
      process_with(this, pid, |proc| {
        let Some(proc) = proc else {
          return; // Never fail when sending to a non-existent PID.
        };

        proc.readonly.send.send(Signal::Send {
          from: this.readonly.mpid,
          data: DynMessage::Term(term),
        });
      });
    }
    ExternalDest::ExternalProc(pid) => {
      todo!("Handle External PID Send - {pid}")
    }
    ExternalDest::InternalName(name) => {
      let Some(pid) = process_whereis(name) else {
        raise!(Error, BadArg, "unregistered name");
      };

      process_with(this, pid, |proc| {
        let Some(proc) = proc else {
          raise!(Error, BadArg, "unregistered name");
        };

        proc.readonly.send.send(Signal::Send {
          from: this.readonly.mpid,
          data: DynMessage::Term(term),
        });
      });
    }
    ExternalDest::ExternalName(name, node) => {
      todo!("Handle External Name Send - {name} @ {node}")
    }
  }
}

pub(crate) async fn process_receive<T>(pid: InternalPid) -> Message<Box<T>>
where
  T: 'static,
{
  ProcMail::receive::<T>(pid).await
}

pub(crate) async fn process_receive_exact<T>(pid: InternalPid) -> Box<T>
where
  T: 'static,
{
  ProcMail::receive_exact::<T>(pid).await
}

pub(crate) async fn process_receive_any(pid: InternalPid) -> DynMessage {
  ProcMail::receive_any(pid).await
}

// -----------------------------------------------------------------------------
// Links, Monitors, Aliases
// -----------------------------------------------------------------------------

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L134
pub(crate) fn process_link<P>(this: &ProcTask, pid: P)
where
  P: ProcessId,
{
  if P::DISTRIBUTED {
    todo!("Handle External PID Link: {pid}")
  }

  let pid: InternalPid = pid.into_internal();

  if this.readonly.mpid == pid {
    return; // Can't link to ourselves
  }

  let Some(dest) = REGISTERED_PROCS.get(pid) else {
    this.readonly.send.send(Signal::Exit {
      from: pid,
      reason: Exit::NOPROC,
      linked: true,
    });
    return;
  };

  let dest: ProcSend = dest.readonly.send.clone();

  match this.internal.lock().links.entry(pid) {
    Entry::Occupied(mut entry) => {
      if entry.get().is_enabled() {
        return; // No need to update an active link
      }

      entry.get_mut().enable();
    }
    Entry::Vacant(entry) => {
      entry.insert(ProcLink::new(dest.downgrade()));
    }
  }

  dest.send(Signal::Link {
    from: this.readonly.mpid,
    send: this.readonly.send.downgrade(),
  });
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L1212
pub(crate) fn process_unlink<P>(this: &ProcTask, pid: P)
where
  P: ProcessId,
{
  if P::DISTRIBUTED {
    todo!("Handle External PID Unlink: {pid}")
  }

  let pid: InternalPid = pid.into_internal();

  let mut guard: MutexGuard<'_, ProcInternal> = this.internal.lock();

  let Some(link) = guard.links.get(&pid) else {
    return; // No link to disable
  };

  if !link.is_enabled() {
    return; // No need to update an inactive link
  }

  let unlink: NonZeroU64 = guard.next_puid();

  let Some(link) = guard.links.get_mut(&pid) else {
    raise!(Error, SysInv, "invalid reborrow");
  };

  link.disable(unlink);

  let Some(send) = link.sender() else {
    return;
  };

  send.send(Signal::Unlink {
    from: this.readonly.mpid,
    send: this.readonly.send.downgrade(),
    ulid: unlink,
  });
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L753
pub(crate) fn process_monitor(this: &ProcTask, item: ExternalDest) -> MonitorRef {
  match item {
    ExternalDest::InternalProc(pid) => {
      let monitor: InternalRef = InternalRef::new_global();

      if this.readonly.mpid == pid {
        return MonitorRef::new(monitor); // Can't monitor ourselves
      }

      if let Some(dest) = REGISTERED_PROCS.get(pid) {
        // TODO: Update local process state

        dest.readonly.send.send(Signal::Monitor {
          from: this.readonly.mpid,
          send: this.readonly.send.downgrade(),
          mref: monitor,
        });
      } else {
        this.readonly.send.send(Signal::MonitorDown {
          mref: monitor,
          item: InternalDest::Proc(pid),
          info: Exit::NOPROC,
        });
      }

      MonitorRef::new(monitor)
    }
    ExternalDest::ExternalProc(pid) => {
      todo!("Handle External PID Monitor - {pid}")
    }
    ExternalDest::InternalName(name) => {
      if let Some(pid) = process_whereis(name) {
        process_monitor(this, ExternalDest::InternalProc(pid))
      } else {
        let monitor: InternalRef = InternalRef::new_global();

        this.readonly.send.send(Signal::MonitorDown {
          mref: monitor,
          item: InternalDest::Name(name),
          info: Exit::NOPROC,
        });

        MonitorRef::new(monitor)
      }
    }
    ExternalDest::ExternalName(name, node) => {
      todo!("Handle External Name Monitor - {name} @ {node}")
    }
  }
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L433
pub(crate) fn process_demonitor(this: &ProcTask, reference: MonitorRef) {
  todo!("Handle Internal Demonitor - {reference} - {this:?}")
}

// -----------------------------------------------------------------------------
// Local Name Registration
//
// BEAM Builtin:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.h
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.c
// -----------------------------------------------------------------------------

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2295
pub(crate) fn process_register(this: &ProcTask, pid: InternalPid, name: Atom) {
  if name == Atom::UNDEFINED {
    raise!(Error, BadArg, "reserved name");
  }

  let mut names: RwLockWriteGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.write();

  let Entry::Vacant(name_entry) = names.entry(name) else {
    raise!(Error, BadArg, "registered name");
  };

  process_with(this, pid, |proc| {
    let Some(proc) = proc else {
      raise!(Error, BadArg, "not alive");
    };

    let mut guard: RwLockWriteGuard<'_, ProcExternal> = proc.external.write();

    if guard.name.is_some() {
      raise!(Error, BadArg, "registered PID");
    }

    guard.name = Some(name);
    name_entry.insert(pid);

    drop(guard);
  });

  drop(names);
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2309
pub(crate) fn process_unregister(this: &ProcTask, name: Atom) {
  let mut names: RwLockWriteGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.write();

  let Entry::Occupied(entry) = names.entry(name) else {
    raise!(Error, BadArg, "not a registered name");
  };

  process_with(this, *entry.get(), |proc| {
    let Some(proc) = proc else {
      return; // Ignore, the PID was just terminated
    };

    let mut guard: RwLockWriteGuard<'_, ProcExternal> = proc.external.write();

    guard.name = None;
    entry.remove();

    drop(guard);
  });

  drop(names);
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2327
pub(crate) fn process_whereis(name: Atom) -> Option<InternalPid> {
  REGISTERED_NAMES.read().get(&name).copied()
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.c#L576
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

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L353
pub(crate) fn process_dict_put(this: &ProcTask, key: Atom, value: Term) -> Option<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.insert(key, value)
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L324
pub(crate) fn process_dict_get(this: &ProcTask, key: Atom) -> Option<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.get(&key)
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L373
pub(crate) fn process_dict_delete(this: &ProcTask, key: Atom) -> Option<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.remove(&key)
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L363
pub(crate) fn process_dict_clear(this: &ProcTask) -> Vec<(Atom, Term)> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.clear()
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L315
pub(crate) fn process_dict_pairs(this: &ProcTask) -> Vec<(Atom, Term)> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.pairs()
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L333
pub(crate) fn process_dict_keys(this: &ProcTask) -> Vec<Atom> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.keys()
}

// BEAM Builtin: N/A
pub(crate) fn process_dict_values(this: &ProcTask) -> Vec<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.values()
}

// -----------------------------------------------------------------------------
// BIFs - Common Utils
// -----------------------------------------------------------------------------

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

pub(crate) fn process_delete(this: &mut ProcTask) {
  tracing::trace!(pid = %this.readonly.mpid, "Proc Delete (1)");

  let mut internal: MutexGuard<'_, ProcInternal> = this.internal.lock();
  let mut external: RwLockWriteGuard<'_, ProcExternal> = this.external.write();

  let exit: Exit = match external.exit.take() {
    Some(data) => data,
    None => Exit::Atom(Atom::new("panic")),
  };

  tracing::trace!(pid = %this.readonly.mpid, ?exit, "Proc Delete (2)");

  if let Some(name) = external.name.take().as_ref() {
    if let None = REGISTERED_NAMES.write().remove(name) {
      tracing::error!(pid = %this.readonly.mpid, %name, "Proc Delete (dangling name)");
    }
  }

  tracing::trace!(pid = %this.readonly.mpid, "Proc Delete (3)");

  for (_pid, state) in internal.links.drain() {
    if !state.is_enabled() {
      continue;
    }

    let Some(send) = state.sender() else {
      continue;
    };

    send.send(Signal::Exit {
      from: this.readonly.mpid,
      reason: exit.clone(),
      linked: true,
    });
  }

  tracing::trace!(pid = %this.readonly.mpid, "Proc Delete (4)");

  if let None = REGISTERED_PROCS.remove(this.readonly.mpid) {
    tracing::error!(pid = %this.readonly.mpid, "Proc Delete (dangling state)");
  };

  tracing::trace!(pid = %this.readonly.mpid, "Proc Delete (5)");

  drop(internal);
  drop(external);
}

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

    let exit: Exit = 'run: loop {
      tokio::select! {
        biased;
        Some(signal) = queue.recv() => {
          let result: Result<Option<Exit>, _> = catch_unwind(AssertUnwindSafe(|| {
            Process::with(|this| process_handle_signal(this, signal))
          }));

          match result.transpose() {
            Some(Ok(exit)) => break 'run exit,
            Some(Err(term)) => break 'run Exit::Term(Term::new_error(term)),
            None => continue 'run,
          }
        }
        result = &mut safe_task => match result {
          Ok(()) => break 'run Exit::NORMAL,
          Err(term) => break 'run Exit::Term(Term::new_error(term)),
        }
      }
    };

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
  // 4. Spawn Task
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
  // 5. Return
  // ---------------------------------------------------------------------------

  SpawnHandle::Process(proc.readonly.mpid)
}

// -----------------------------------------------------------------------------
// BIFs - Signal Handling
// -----------------------------------------------------------------------------

fn process_handle_signal(this: &ProcTask, signal: Signal) -> Option<Exit> {
  let mut guard: MutexGuard<'_, ProcInternal> = this.internal.lock();

  match signal {
    Signal::Exit {
      from,
      reason,
      linked: false,
    } => {
      tracing::trace!(pid = %this.readonly.mpid, %from, %reason, "Proc Exit");

      match reason {
        Exit::Atom(atom) if atom == Atom::NORMAL => {
          if guard.flags.contains(ProcessFlags::TRAP_EXIT) {
            tracing::trace!("Proc Exit (trap)");
            this.readonly.send.send(Signal::Send {
              from,
              data: DynMessage::Exit(ExitMessage::new(from, reason)),
            });
          } else if this.readonly.mpid == from {
            tracing::trace!("Proc Exit (terminate)");
            return Some(reason);
          } else {
            tracing::trace!("Proc Exit (ignore)");
          }
        }
        Exit::Atom(atom) if atom == Atom::KILLED => {
          tracing::trace!("Proc Exit (terminate)");
          return Some(reason);
        }
        Exit::Atom(_) | Exit::Term(_) => {
          if guard.flags.contains(ProcessFlags::TRAP_EXIT) {
            tracing::trace!("Proc Exit (trap)");
            this.readonly.send.send(Signal::Send {
              from,
              data: DynMessage::Exit(ExitMessage::new(from, reason)),
            });
          } else {
            tracing::trace!("Proc Exit (terminate)");
            return Some(reason);
          }
        }
      }
    }

    // Exit signal handling (linked):
    //
    // If the link state exists and the process is trapping exits, the signal
    // is converted to an `ExitMessage` and delivered to the inbox.
    // If the reason is not normal, the receiving process is terminated.
    Signal::Exit {
      from,
      reason,
      linked: true,
    } => {
      tracing::trace!(pid = %this.readonly.mpid, %from, %reason, "Proc Link Exit");

      let Some(state) = guard.links.remove(&from) else {
        tracing::trace!("Proc Link Exit (dead)");
        return None;
      };

      if state.is_enabled() {
        if guard.flags.contains(ProcessFlags::TRAP_EXIT) {
          tracing::trace!("Proc Link Exit (trap)");
          this.readonly.send.send(Signal::Send {
            from: this.readonly.mpid,
            data: DynMessage::Exit(ExitMessage::new(from, reason)),
          });
        } else if reason.is_normal() {
          tracing::trace!("Proc Link Exit (ignore)");
        } else {
          tracing::trace!("Proc Link Exit (terminate)");
          return Some(reason);
        }
      } else {
        tracing::trace!("Proc Link Exit (disabled)");
      }
    }

    // Send signal handling:
    //
    // The content of the message is moved to the internal inbox buffer.
    Signal::Send { from, data } => {
      tracing::trace!(pid = %this.readonly.mpid, %from, "Proc Send");
      guard.inbox.push(data);
    }

    // Link signal handling:
    //
    // If no link state exists for the sender, a default enabled state is
    // inserted; otherwise, the signal is dropped.
    Signal::Link { from, send } => {
      tracing::trace!(pid = %this.readonly.mpid, %from, "Proc Link");

      match guard.links.entry(from) {
        Entry::Occupied(_) => {
          tracing::trace!("Proc Link (occupied)");
        }
        Entry::Vacant(entry) => {
          tracing::trace!("Proc Link (vacant)");
          entry.insert(ProcLink::new(send));
        }
      }
    }

    // Unlink signal handling:
    //
    // If the link state exists and is enabled, it is removed and the sender
    // receives an `UnlinkAck` signal. If not, the signal is dropped.
    Signal::Unlink { from, send, ulid } => {
      tracing::trace!(pid = %this.readonly.mpid, %from, %ulid, "Proc Unlink");

      match guard.links.entry(from) {
        Entry::Occupied(entry) => {
          tracing::trace!("Proc Unlink (occupied)");

          if entry.get().is_disabled() {
            tracing::trace!("Proc Unlink (disabled)");
            return None;
          }

          let _ignore: ProcLink = entry.remove();

          if let Some(send) = send.upgrade() {
            tracing::trace!("Proc Unlink (ack)");
            send.send(Signal::UnlinkAck {
              from: this.readonly.mpid,
              ulid,
            });
          } else {
            tracing::trace!("Proc Unlink (dead)");
          }
        }
        Entry::Vacant(_) => {
          tracing::trace!("Proc Unlink (vacant)");
        }
      }
    }

    // UnlinkAck signal handling:
    //
    // If the link state exists, is enabled, and the given `ulid` matches,
    // the link state is removed. Otherwise, the signal is dropped.
    Signal::UnlinkAck { from, ulid } => {
      tracing::trace!(pid = %this.readonly.mpid, %from, %ulid, "Proc UnlinkAck");

      match guard.links.entry(from) {
        Entry::Occupied(entry) => {
          tracing::trace!("Proc UnlinkAck (occupied)");

          let state: &ProcLink = entry.get();

          if state.is_disabled() {
            if state.matches(ulid) {
              tracing::trace!("Proc UnlinkAck (remove)");
              let _ignore: ProcLink = entry.remove();
            } else {
              tracing::trace!("Proc UnlinkAck (stale)");
            }
          } else {
            tracing::trace!("Proc UnlinkAck (enabled)");
          }
        }
        Entry::Vacant(_) => {
          tracing::trace!("Proc UnlinkAck (vacant)");
        }
      }
    }

    Signal::Monitor { from, send, mref } => {
      todo!("Handle Signal::Monitor {from}, {send:?}, {mref}")
    }
    Signal::MonitorDown { mref, item, info } => {
      todo!("Handle Signal::MonitorDown {mref}, {item}, {info}")
    }
    Signal::Demonitor { from, mref } => {
      todo!("Handle Signal::Demonitor {from}, {mref}")
    }
  }

  None
}

// -----------------------------------------------------------------------------
// Misc. Internal Utils
// -----------------------------------------------------------------------------

fn process_with<F, R>(this: &ProcTask, pid: InternalPid, f: F) -> R
where
  F: FnOnce(Option<&ProcData>) -> R,
{
  let hold: Arc<ProcData>;
  let data: &ProcData;

  if this.readonly.mpid == pid {
    data = this;
  } else {
    let Some(context) = REGISTERED_PROCS.get(pid) else {
      return f(None);
    };

    hold = context;
    data = &hold;
  }

  f(Some(data))
}
