// OTP COMMIT: 11c6025cba47e24950cd4b4fc9f7e9e522388542
use hashbrown::HashMap;
use hashbrown::hash_map::Entry;
use parking_lot::RwLock;
use parking_lot::RwLockReadGuard;
use parking_lot::RwLockUpgradableReadGuard;
use parking_lot::RwLockWriteGuard;
use std::mem::MaybeUninit;
use std::panic::AssertUnwindSafe;
use std::sync::LazyLock;
use tokio::task;
use tokio::task::futures::TaskLocalFuture;
use triomphe::Arc;

use crate::core::CatchUnwind;
use crate::core::raise;
use crate::erts::DynMessage;
use crate::erts::Message;
use crate::erts::Process;
use crate::erts::ProcessData;
use crate::erts::ProcessDict;
use crate::erts::ProcessFlags;
use crate::erts::ProcessInfo;
use crate::erts::ProcessRecv;
use crate::erts::ProcessSend;
use crate::erts::ProcessSlot;
use crate::erts::ProcessTable;
use crate::erts::ProcessTask;
use crate::erts::Runtime;
use crate::erts::Signal;
use crate::erts::SpawnConfig;
use crate::erts::SpawnHandle;
use crate::erts::unbounded_channel;
use crate::lang::Atom;
use crate::lang::ExitReason;
use crate::lang::ExternalDest;
use crate::lang::ExternalPid;
use crate::lang::InternalPid;
use crate::lang::RawPid;
use crate::lang::Term;

// A table mapping internal process identifiers to process data.
static REGISTERED_PROCS: LazyLock<ProcessTable<ProcessSlot>> =
  LazyLock::new(|| ProcessTable::with_capacity(Runtime::CAP_REGISTERED_PROCS));

// A table mapping registered names to internal process identifiers.
static REGISTERED_NAMES: LazyLock<RwLock<HashMap<Atom, InternalPid>>> =
  LazyLock::new(|| RwLock::new(HashMap::with_capacity(Runtime::CAP_REGISTERED_NAMES)));

// -----------------------------------------------------------------------------
// Common Utilities
// -----------------------------------------------------------------------------

pub(crate) fn translate_pid(pid: RawPid) -> Option<(u32, u32)> {
  if pid.into_bits() & RawPid::TAG_MASK == RawPid::TAG_DATA {
    Some(REGISTERED_PROCS.translate_pid(pid))
  } else {
    None
  }
}

pub(crate) fn process_delete(pid: InternalPid) -> Option<Arc<ProcessSlot>> {
  // ---------------------------------------------------------------------------
  // 1. Remove Process
  // ---------------------------------------------------------------------------

  let Some(slot) = REGISTERED_PROCS.remove(pid.bits()) else {
    return None;
  };

  // ---------------------------------------------------------------------------
  // 2. Lock Process
  // ---------------------------------------------------------------------------

  let proc_guard: RwLockWriteGuard<'_, ProcessData> = slot.data.write();

  // ---------------------------------------------------------------------------
  // 3. Unregister Process Name
  // ---------------------------------------------------------------------------

  if let Some(name) = proc_guard.name.as_ref() {
    if let None = REGISTERED_NAMES.write().remove(name) {
      tracing::warn!(%name, %pid, "Dangling process name");
    }
  }

  // ---------------------------------------------------------------------------
  // 4. Unlock Process
  // ---------------------------------------------------------------------------

  drop(proc_guard);

  // ---------------------------------------------------------------------------
  // 5. Return
  // ---------------------------------------------------------------------------

  Some(slot)
}

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
    data.push(InternalPid::new(pid));
  }

  data
}

// BEAM Builtin: N/A
pub(crate) fn process_get_flags(process: &ProcessTask) -> ProcessFlags {
  let guard: RwLockReadGuard<'_, ProcessData> = process.data.read();
  let value: ProcessFlags = guard.flags;

  drop(guard);

  value
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2013
pub(crate) fn process_set_flags(process: &ProcessTask, flags: ProcessFlags) {
  process.data.write().flags = flags;
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2013
pub(crate) fn process_set_flag(process: &ProcessTask, flag: ProcessFlags, value: bool) {
  process.data.write().flags.set(flag, value);
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_bif_info.c#L1558
pub(crate) fn process_info(process: &ProcessTask, pid: InternalPid) -> Option<ProcessInfo> {
  // ---------------------------------------------------------------------------
  // 1. Find Process
  // ---------------------------------------------------------------------------

  let hold: Arc<ProcessSlot>;
  let this: &ProcessSlot;

  if process.mpid == pid {
    this = process;
  } else {
    let Some(context) = REGISTERED_PROCS.get(pid.bits()) else {
      return None;
    };

    hold = context;
    this = &hold;
  }

  // ---------------------------------------------------------------------------
  // 2. Lock Process
  // ---------------------------------------------------------------------------

  let guard: RwLockReadGuard<'_, ProcessData> = this.data.read();

  // ---------------------------------------------------------------------------
  // 3. Gather Process Info
  // ---------------------------------------------------------------------------

  let info: ProcessInfo = ProcessInfo {
    async_dist: guard.flags.contains(ProcessFlags::ASYNC_DIST),
    dictionary: HashMap::from_iter(this.dict.pairs()),
    pid_group_leader: guard.group_leader,
    pid_spawn_parent: this.root,
    registered_name: guard.name,
    trap_exit: guard.flags.contains(ProcessFlags::TRAP_EXIT),
  };

  // ---------------------------------------------------------------------------
  // 4. Unlock Process
  // ---------------------------------------------------------------------------

  drop(guard);

  // ---------------------------------------------------------------------------
  // 5. Return
  // ---------------------------------------------------------------------------

  Some(info)
}

// -----------------------------------------------------------------------------
// Spawning & Messaging
// -----------------------------------------------------------------------------

pub(crate) fn process_spawn<F>(
  process: &ProcessTask,
  options: SpawnConfig,
  future: F,
) -> SpawnHandle
where
  F: Future<Output = ()> + Send + 'static,
{
  process_spawn_internal(Some(process), options, future)
}

pub(crate) fn process_spawn_root<F>(future: F) -> InternalPid
where
  F: Future<Output = ()> + Send + 'static,
{
  match process_spawn_internal(None, SpawnConfig::new(), future) {
    SpawnHandle::Process(process) => process,
    SpawnHandle::Monitor(_, _) => unreachable!(),
  }
}

fn process_spawn_internal<F>(
  process: Option<&ProcessTask>,
  options: SpawnConfig,
  future: F,
) -> SpawnHandle
where
  F: Future<Output = ()> + Send + 'static,
{
  // ---------------------------------------------------------------------------
  // 1. Setup Context
  // ---------------------------------------------------------------------------

  // Insert a dummy PID for now, the real PID will be given if the insert succeeds.
  let mut target_pid: InternalPid = InternalPid::new(RawPid::from_bits(0));
  let mut leader_pid: InternalPid = target_pid;
  let parent_pid: Option<InternalPid> = process.map(|process| process.mpid);

  // Initialize default flags from user-provided options.
  let flags: ProcessFlags = {
    let mut flags: ProcessFlags = ProcessFlags::empty();
    flags.set(ProcessFlags::TRAP_EXIT, options.trap_exit);
    flags.set(ProcessFlags::ASYNC_DIST, options.async_dist);
    flags
  };

  let data: ProcessData = ProcessData {
    flags,
    exit: None,
    name: None,
    task: None,
    group_leader: leader_pid,
    inbox_buffer: Vec::with_capacity(Runtime::CAP_PROC_MSG_BUFFER),
  };

  let dict: ProcessDict = ProcessDict::new();

  let (sig_send, sig_recv): (ProcessSend, ProcessRecv) = unbounded_channel();

  // ---------------------------------------------------------------------------
  // 2. Create Process State
  // ---------------------------------------------------------------------------

  let initialize = |uninit: &mut MaybeUninit<ProcessSlot>, pid: RawPid| {
    target_pid = InternalPid::new(pid);
    leader_pid = parent_pid.unwrap_or(target_pid);

    let slot: *mut ProcessSlot = uninit.as_mut_ptr();

    let data: RwLock<ProcessData> = RwLock::new(ProcessData {
      group_leader: leader_pid,
      ..data
    });

    unsafe {
      (&raw mut (*slot).mpid).write(target_pid);
      (&raw mut (*slot).send).write(sig_send);
      (&raw mut (*slot).root).write(parent_pid);
      (&raw mut (*slot).data).write(data);
      (&raw mut (*slot).dict).write(dict);
    }
  };

  let Ok(slot) = REGISTERED_PROCS.insert(initialize) else {
    raise!(Error, SysCap, "too many processes");
  };

  // ---------------------------------------------------------------------------
  // 3. Create Process Task
  // ---------------------------------------------------------------------------

  let context: ProcessTask = ProcessTask {
    slot: Arc::clone(&slot),
  };

  let local: TaskLocalFuture<ProcessTask, _> = Process::scope(context, async move {
    let mut queue: ProcessRecv = sig_recv;
    let safe_task: CatchUnwind<AssertUnwindSafe<F>> = CatchUnwind::new(AssertUnwindSafe(future));

    tokio::pin!(safe_task);

    let reason: ExitReason = 'run: loop {
      tokio::select! {
        biased;
        Some(signal) = queue.recv() => {
          match signal {
            Signal::Message(from, term) => {
              tracing::info!(%from, %term, "Signal::Message");

              Process::with(|this| {
                this.data.write().inbox_buffer.push(DynMessage::Term(term));
              });
            }
          }
        }
        result = &mut safe_task => match result {
          Ok(()) => break 'run ExitReason::Normal,
          Err(term) => break 'run ExitReason::Term(Term::new_error(term)),
        }
      }
    };

    Process::with(|this| {
      let mut guard: RwLockWriteGuard<'_, ProcessData> = this.data.write();

      debug_assert!(guard.exit.is_none());

      guard.exit = Some(reason);

      drop(guard);
    });
  });

  // ---------------------------------------------------------------------------
  // 4. Link State <-> Task
  // ---------------------------------------------------------------------------

  if let Some(parent) = parent_pid {
    tracing::event!(
      target: "errai",
      tracing::Level::INFO,
      target = %target_pid,
      %parent,
      "Process spawn",
    );
  } else {
    tracing::event!(
      target: "errai",
      tracing::Level::INFO,
      target = %target_pid,
      "Process spawn",
    );
  }

  slot.data.write().task = Some(task::spawn(local));

  // ---------------------------------------------------------------------------
  // 5. Return
  // ---------------------------------------------------------------------------

  SpawnHandle::Process(target_pid)
}

pub(crate) fn process_send(from: &ProcessTask, dest: ExternalDest, term: Term) {
  match dest {
    ExternalDest::InternalProc(pid) => process_send_internal_proc(from, pid, term),
    ExternalDest::ExternalProc(pid) => process_send_external_proc(from, pid, term),
    ExternalDest::InternalName(name) => process_send_internal_name(from, name, term),
    ExternalDest::ExternalName(name, node) => process_send_external_name(from, name, node, term),
  }
}

fn process_send_internal_proc(from: &ProcessTask, ipid: InternalPid, term: Term) {
  let Some(dest) = REGISTERED_PROCS.get(ipid.bits()) else {
    return; // Never fail when sending to a non-existent PID.
  };

  if let Err(_error) = dest.send.send(Signal::Message(from.mpid, term)) {
    raise!(Error, SysInv, "sending on a closed channel");
  }
}

fn process_send_external_proc(from: &ProcessTask, epid: ExternalPid, term: Term) {
  todo!("process_send_external_proc")
}

fn process_send_internal_name(from: &ProcessTask, name: Atom, term: Term) {
  if let Some(pid) = process_whereis(name) {
    process_send_internal_proc(from, pid, term);
  } else {
    raise!(Error, BadArg, "unregistered name");
  }
}

fn process_send_external_name(from: &ProcessTask, name: Atom, node: Atom, term: Term) {
  todo!("process_send_external_name")
}

pub(crate) fn process_receive<T>(
  process: &ProcessTask,
) -> impl Future<Output = Message<Box<T>>> + use<T>
where
  T: 'static,
{
  let pid: InternalPid = process.mpid;

  async move {
    let poll_message: DynMessage = process_poll(pid, DynMessage::is::<T>).await;

    // SAFETY: `DynMessage::is` ensures the message type is valid.
    let cast_message: Message<Box<T>> = unsafe { poll_message.downcast_unchecked() };

    cast_message
  }
}

pub(crate) fn process_receive_exact<T>(
  process: &ProcessTask,
) -> impl Future<Output = Box<T>> + use<T>
where
  T: 'static,
{
  let pid: InternalPid = process.mpid;

  async move {
    let poll_message: DynMessage = process_poll(pid, DynMessage::is_exact::<T>).await;

    // SAFETY: `DynMessage::is_exact` ensures the message type is valid.
    let cast_message: Box<T> = unsafe { poll_message.downcast_exact_unchecked() };

    cast_message
  }
}

pub(crate) async fn process_poll<F>(pid: InternalPid, filter: F) -> DynMessage
where
  F: Fn(&DynMessage) -> bool,
{
  let value: Option<DynMessage> = Process::with(|this| {
    debug_assert!(this.mpid == pid);
    process_poll_inbox(this, &filter)
  });

  if let Some(message) = value {
    return message;
  }

  'poll: loop {
    task::yield_now().await;

    let value: Option<DynMessage> = Process::with(|this| {
      debug_assert!(this.mpid == pid);
      process_poll_inbox(this, &filter)
    });

    if let Some(message) = value {
      break 'poll message;
    }
  }
}

fn process_poll_inbox<F>(process: &ProcessTask, filter: F) -> Option<DynMessage>
where
  F: Fn(&DynMessage) -> bool,
{
  let guard: RwLockUpgradableReadGuard<'_, ProcessData> = process.data.upgradable_read();
  let index: Option<usize> = guard.inbox_buffer.iter().position(filter);

  let Some(index) = index else {
    return None;
  };

  let mut guard: RwLockWriteGuard<'_, ProcessData> = RwLockUpgradableReadGuard::upgrade(guard);
  let dyn_value: DynMessage = guard.inbox_buffer.remove(index);

  drop(guard);

  Some(dyn_value)
}

// -----------------------------------------------------------------------------
// Local Name Registration
//
// BEAM Builtin:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.h
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.c
// -----------------------------------------------------------------------------

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2295
pub(crate) fn process_register(process: &ProcessTask, pid: InternalPid, name: Atom) {
  // ---------------------------------------------------------------------------
  // 1. Validate
  // ---------------------------------------------------------------------------

  if name == Atom::UNDEFINED {
    raise!(Error, BadArg, "reserved name");
  }

  // ---------------------------------------------------------------------------
  // 2. Lock Name Registry
  // ---------------------------------------------------------------------------

  let mut name_guard: RwLockWriteGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.write();

  // ---------------------------------------------------------------------------
  // 3. Find Entry
  // ---------------------------------------------------------------------------

  let Entry::Vacant(name_entry) = name_guard.entry(name) else {
    raise!(Error, BadArg, "registered name");
  };

  // ---------------------------------------------------------------------------
  // 3. Find Process
  // ---------------------------------------------------------------------------

  let hold: Arc<ProcessSlot>;
  let this: &ProcessSlot;

  if process.mpid == pid {
    this = process;
  } else {
    let Some(context) = REGISTERED_PROCS.get(pid.bits()) else {
      raise!(Error, BadArg, "PID not alive");
    };

    hold = context;
    this = &hold;
  }

  // ---------------------------------------------------------------------------
  // 4. Lock Process
  // ---------------------------------------------------------------------------

  let mut proc_guard: RwLockWriteGuard<'_, ProcessData> = this.data.write();

  // ---------------------------------------------------------------------------
  // 4. Update Process/Registry
  // ---------------------------------------------------------------------------

  if proc_guard.name.is_some() {
    raise!(Error, BadArg, "registered PID");
  }

  proc_guard.name = Some(name);
  name_entry.insert(pid);

  // ---------------------------------------------------------------------------
  // 5. Unlock Process/Registry
  // ---------------------------------------------------------------------------

  drop(proc_guard);
  drop(name_guard);
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2309
pub(crate) fn process_unregister(process: &ProcessTask, name: Atom) {
  // ---------------------------------------------------------------------------
  // 1. Lock Name Registry
  // ---------------------------------------------------------------------------

  let mut name_guard: RwLockWriteGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.write();

  // ---------------------------------------------------------------------------
  // 2. Find Entry
  // ---------------------------------------------------------------------------

  let Entry::Occupied(name_entry) = name_guard.entry(name) else {
    raise!(Error, BadArg, "unregistered name");
  };

  // ---------------------------------------------------------------------------
  // 3. Find Process
  // ---------------------------------------------------------------------------

  let hold: Arc<ProcessSlot>;
  let this: &ProcessSlot;

  if process.mpid == *name_entry.get() {
    this = process;
  } else {
    let Some(context) = REGISTERED_PROCS.get(name_entry.get().bits()) else {
      // TODO: This should be unreachable (?)
      raise!(Error, BadArg, "PID not alive");
    };

    hold = context;
    this = &hold;
  }

  // ---------------------------------------------------------------------------
  // 4. Lock Process
  // ---------------------------------------------------------------------------

  let mut proc_guard: RwLockWriteGuard<'_, ProcessData> = this.data.write();

  // ---------------------------------------------------------------------------
  // 4. Update Process/Registry
  // ---------------------------------------------------------------------------

  proc_guard.name = None;
  name_entry.remove();

  // ---------------------------------------------------------------------------
  // 5. Unlock Process/Registry
  // ---------------------------------------------------------------------------

  drop(proc_guard);
  drop(name_guard);
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/bif.c#L2327
pub(crate) fn process_whereis(name: Atom) -> Option<InternalPid> {
  let guard: RwLockReadGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.read();
  let value: Option<InternalPid> = guard.get(&name).copied();

  drop(guard);

  value
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/register.c#L576
pub(crate) fn process_registered() -> Vec<Atom> {
  let guard: RwLockReadGuard<'_, HashMap<Atom, InternalPid>> = REGISTERED_NAMES.read();
  let value: Vec<Atom> = Vec::from_iter(guard.keys().copied());

  drop(guard);

  value
}

// -----------------------------------------------------------------------------
// Process Dictionary
//
// BEAM Builtin:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.h
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c
// -----------------------------------------------------------------------------

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L353
pub(crate) fn process_dict_put(process: &ProcessTask, key: Atom, value: Term) -> Option<Term> {
  process.dict.insert(key, value)
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L324
pub(crate) fn process_dict_get(process: &ProcessTask, key: Atom) -> Option<Term> {
  process.dict.get(&key)
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L373
pub(crate) fn process_dict_delete(process: &ProcessTask, key: Atom) -> Option<Term> {
  process.dict.remove(&key)
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L363
pub(crate) fn process_dict_clear(process: &ProcessTask) -> Vec<(Atom, Term)> {
  process.dict.clear()
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L315
pub(crate) fn process_dict_pairs(process: &ProcessTask) -> Vec<(Atom, Term)> {
  process.dict.pairs()
}

// BEAM Builtin: https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L333
pub(crate) fn process_dict_keys(process: &ProcessTask) -> Vec<Atom> {
  process.dict.keys()
}

// BEAM Builtin: N/A
pub(crate) fn process_dict_values(process: &ProcessTask) -> Vec<Term> {
  process.dict.values()
}
