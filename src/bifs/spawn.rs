// -----------------------------------------------------------------------------
// Process Spawning
// -----------------------------------------------------------------------------

use parking_lot::RwLock;
use parking_lot::RwLockWriteGuard;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::panic::AssertUnwindSafe;
use tokio::task;
use tokio::task::futures::TaskLocalFuture;
use tracing::Span;
use tracing::span;
use triomphe::Arc;

use crate::bifs;
use crate::core::Atom;
use crate::core::Exit;
use crate::core::ExternalDest;
use crate::core::InternalPid;
use crate::core::MonitorRef;
use crate::core::ProcessId;
use crate::core::Term;
use crate::erts::Process;
use crate::erts::ProcessFlags;
use crate::erts::Signal;
use crate::erts::SignalRecv;
use crate::erts::SpawnConfig;
use crate::erts::SpawnHandle;
use crate::proc;
use crate::proc::ProcData;
use crate::proc::ProcExternal;
use crate::proc::ProcInternal;
use crate::proc::ProcReadOnly;
use crate::proc::ProcRecv;
use crate::proc::ProcSend;
use crate::proc::ProcTask;
use crate::proc::TaskGuard;
use crate::raise;
use crate::utils::CatchUnwind;

use super::REGISTERED_NAMES;
use super::REGISTERED_PROCS;

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
pub(crate) fn proc_exit<P>(this: &ProcTask, pid: P, exit: Exit)
where
  P: ProcessId,
{
  if P::DISTRIBUTED {
    todo!("Handle External PID Exit: {pid}")
  }

  let pid: InternalPid = pid.into_internal();

  bifs::proc_with(this, pid, |proc| {
    let Some(proc) = proc else {
      return; // Never fail when sending to a non-existent PID.
    };

    proc.readonly.send_exit(this.readonly.mpid, exit);
  });
}

/// Spawns a new process with the given configuration.
///
/// The spawned process:
///
/// 1. Executes the provided future
/// 2. Has its own signal queue and state
/// 3. May be linked/monitored based on `opts`
///
/// Returns a handle containing the PID and optional monitor reference.
pub(crate) fn proc_spawn<F>(this: &ProcTask, opts: SpawnConfig, future: F) -> SpawnHandle
where
  F: Future<Output = ()> + Send + 'static,
{
  proc_spawn_internal(this.into(), future, opts)
}

/// Spawns the root supervisor process.
///
/// The root process has no parent and is used to supervise the entire
/// application. It must not be spawned with monitoring enabled.
pub(crate) fn proc_spawn_root<F>(future: F) -> InternalPid
where
  F: Future<Output = ()> + Send + 'static,
{
  match proc_spawn_internal(None, future, SpawnConfig::new()) {
    SpawnHandle::Process(pid) => pid,
    SpawnHandle::Monitor(_, _) => raise!(Error, SysInv, "root process monitor"),
  }
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
fn proc_spawn_internal<F>(parent: Option<&ProcTask>, future: F, options: SpawnConfig) -> SpawnHandle
where
  F: Future<Output = ()> + Send + 'static,
{
  // ---------------------------------------------------------------------------
  // 1. Create Process
  // ---------------------------------------------------------------------------

  let (proc, sig_recv): (Arc<ProcData>, ProcRecv) = proc_create(parent, options);

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
        if let Some(exit) = Process::with(|this| proc_handle_signal(this, signal)) {
          break 'run exit;
        }
      }

      tokio::select! {
        biased;
        // Prioritize signals over user code
        Some(signal) = queue.recv() => {
          if let Some(exit) = Process::with(|this| proc_handle_signal(this, signal)) {
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

    bifs::proc_link(root, proc.readonly.mpid);
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
    let monitor: MonitorRef = bifs::proc_monitor(root, mtarget);
    let mhandle: SpawnHandle = SpawnHandle::Monitor(proc.readonly.mpid, monitor);

    handle = mhandle;
  }

  // ---------------------------------------------------------------------------
  // 5. Spawn Task
  // ---------------------------------------------------------------------------

  if let Some(root) = parent {
    tracing::debug!(pid = %proc.readonly.mpid, from = %root.readonly.mpid, "Proc Spawn");
  } else {
    tracing::debug!(pid = %proc.readonly.mpid, "Proc Spawn");
  }

  let Ok(()) = proc.readonly.task.set(task::spawn(local)) else {
    raise!(Error, SysInv, "duplicate task");
  };

  // ---------------------------------------------------------------------------
  // 6. Return
  // ---------------------------------------------------------------------------

  handle
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
fn proc_create(parent: Option<&ProcTask>, options: SpawnConfig) -> (Arc<ProcData>, ProcRecv) {
  let parent_pid: Option<InternalPid> = parent.map(|process| process.readonly.mpid);
  let leader_pid: Option<InternalPid> = parent.map(|process| process.internal().group_leader);

  let (sig_send, sig_recv): (ProcSend, ProcRecv) = proc::unbounded_channel();

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
      (&raw mut (*proc_state).internal).write(UnsafeCell::new(internal));
      (&raw mut (*proc_state).external).write(RwLock::new(external));
    }
  };

  let Ok(proc) = REGISTERED_PROCS.insert(initialize) else {
    raise!(Error, SysCap, "too many processes");
  };

  (proc, sig_recv)
}

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
pub(crate) fn proc_remove(this: &mut ProcTask) {
  let span: Span = tracing::trace_span!("Proc Delete", pid = %this.readonly.mpid);
  let span_guard: span::Entered<'_> = span.enter();

  tracing::trace!("(1) - Lock");

  let mut internal: TaskGuard<'_, ProcInternal> = this.internal();
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

    if let Some(proc) = bifs::proc_find(pid) {
      proc
        .readonly
        .send_link_exit(this.readonly.mpid, exit.clone());
    }
  }

  tracing::trace!("(4) - Send Monitor `DOWN`");

  for (mref, state) in internal.monitor_recv.drain() {
    if let Some(proc) = bifs::proc_find(state.origin()) {
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

/// Processes a signal in the context of the receiving process.
///
/// Returns `Some(Exit)` if the signal should terminate the process.
fn proc_handle_signal(this: &ProcTask, signal: Signal) -> Option<Exit> {
  signal.recv(&this.readonly, &mut this.internal())
}
