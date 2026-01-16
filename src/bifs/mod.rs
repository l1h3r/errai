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
use std::sync::LazyLock;
use std::time::Duration;
use tokio::task;
use tokio::task::futures::TaskLocalFuture;
use tracing::Span;
use tracing::span;
use triomphe::Arc;

use crate::core::CatchUnwind;
use crate::core::ProcData;
use crate::core::ProcExternal;
use crate::core::ProcInternal;
use crate::core::ProcLink;
use crate::core::ProcMail;
use crate::core::ProcMonitor;
use crate::core::ProcReadOnly;
use crate::core::ProcRecv;
use crate::core::ProcSend;
use crate::core::ProcTask;
use crate::core::ProcessFlags;
use crate::core::ProcessInfo;
use crate::core::raise;
use crate::core::unbounded_channel;
use crate::erts::DynMessage;
use crate::erts::Message;
use crate::erts::ProcTable;
use crate::erts::Process;
use crate::erts::Runtime;
use crate::erts::Signal;
use crate::erts::SignalRecv;
use crate::erts::SpawnConfig;
use crate::erts::SpawnHandle;
use crate::lang::Atom;
use crate::lang::Exit;
use crate::lang::ExternalDest;
use crate::lang::InternalDest;
use crate::lang::InternalPid;
use crate::lang::MonitorRef;
use crate::lang::ProcessId;
use crate::lang::Term;
use crate::lang::TimerRef;

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

pub(crate) fn process_alive(_this: &ProcTask, _pid: InternalPid) -> bool {
  todo!("Handle Process Alive")
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
  todo!("Handle Process Info")
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

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L1212
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

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L753
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

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L433
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
      entry.remove();
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
  let span: Span = tracing::trace_span!("Proc Delete", pid = %this.readonly.mpid);
  let span_guard: span::Entered<'_> = span.enter();

  tracing::trace!("(1) - Lock");

  let mut internal: MutexGuard<'_, ProcInternal> = this.internal.lock();
  let mut external: RwLockWriteGuard<'_, ProcExternal> = this.external.write();

  let exit: Exit = match external.exit.take() {
    Some(data) => data,
    None => Exit::Atom(Atom::new("panic")),
  };

  tracing::trace!(?exit, "(2) - Unregister");

  if let Some(name) = external.name.take().as_ref() {
    if let None = REGISTERED_NAMES.write().remove(name) {
      tracing::error!(%name, "dangling name");
    }
  }

  tracing::trace!("(3) - Links");

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

  tracing::trace!("(4) - Monitors");

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
  drop(external);

  drop(span_guard);
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
      while let Ok(signal) = queue.try_recv() {
        if let Some(exit) = Process::with(|this| process_handle_signal(this, signal)) {
          break 'run exit;
        }
      }

      tokio::select! {
        biased;
        Some(signal) = queue.recv() => {
          if let Some(exit) = Process::with(|this| process_handle_signal(this, signal)) {
            break 'run exit;
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

pub(crate) fn process_find(pid: InternalPid) -> Option<Arc<ProcData>> {
  REGISTERED_PROCS.get(pid)
}

fn process_handle_signal(this: &ProcTask, signal: Signal) -> Option<Exit> {
  signal.recv(&this.readonly, &mut this.internal.lock())
}

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
