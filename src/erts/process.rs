use bitflags::bitflags;
use std::time::Duration;
use tokio::task;
use tokio::task::futures::TaskLocalFuture;
use tokio::time;

use crate::bifs;
use crate::core::ProcTask;
use crate::core::ProcessFlags;
use crate::core::ProcessInfo;
use crate::core::raise;
use crate::erts::DynMessage;
use crate::erts::Message;
use crate::erts::SpawnConfig;
use crate::erts::SpawnHandle;
use crate::lang::Atom;
use crate::lang::Exit;
use crate::lang::ExternalDest;
use crate::lang::InternalDest;
use crate::lang::InternalPid;
use crate::lang::InternalRef;
use crate::lang::Item;
use crate::lang::Term;

mod process_data;
mod process_dict;
mod process_id;
mod process_info;
mod process_table;

pub(crate) use self::process_data::ProcessData;
pub(crate) use self::process_data::ProcessSlot;
pub(crate) use self::process_data::ProcessTask;
pub(crate) use self::process_dict::ProcessDict;

pub use self::process_id::ProcessId;
pub use self::process_info::ProcessInfo;
pub use self::process_table::ProcessTable;
pub use self::process_table::ProcessTableFull;

// -----------------------------------------------------------------------------
// @alias - References
// -----------------------------------------------------------------------------

pub type MonitorRef = DynRef;

pub type AliasRef = InternalRef;
pub type TimerRef = InternalRef;

// -----------------------------------------------------------------------------
// @data - Task Globals
// -----------------------------------------------------------------------------

tokio::task_local! {
  static CONTEXT: ProcessTask;
}

// -----------------------------------------------------------------------------
// @type - ProcessFlags
//
// Somewhat copied from:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process.h#L1632
// -----------------------------------------------------------------------------

bitflags! {
  #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
  pub struct ProcessFlags: u32 {
    const TRAP_EXIT  = 1 << 22;
    const ASYNC_DIST = 1 << 26;
  }
}

// -----------------------------------------------------------------------------
// @api - Process
// -----------------------------------------------------------------------------

/// Errai process API.
pub struct Process;

impl Process {
  /// Sets the task-local process context.
  #[inline]
  pub(crate) fn scope<F>(task: ProcessTask, future: F) -> TaskLocalFuture<ProcessTask, F>
  where
    F: Future,
  {
    CONTEXT.scope(task, future)
  }

  /// Accesses the current task-local process context and runs the given function.
  #[inline]
  pub(crate) fn with<F, R>(f: F) -> R
  where
    F: FnOnce(&ProcessTask) -> R,
  {
    match CONTEXT.try_with(f) {
      Ok(result) => result,
      Err(_error) => raise!(Error, SysInv, "task-local value not set"),
    }
  }

  // ---------------------------------------------------------------------------
  // General API
  // ---------------------------------------------------------------------------

  /// Returns the process identifier of the calling process.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#self/0>
  pub fn this() -> InternalPid {
    Self::with(|this| this.mpid)
  }

  /// Returns a list of process identifiers corresponding to all the
  /// processes currently existing on the local node.
  ///
  /// Notice that an exiting process exists, but is not alive. That is,
  /// [`Process::alive`] returns false for an exiting process, but its process
  /// identifier is part of the result returned from [`Process::list`].
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#processes/0>
  pub fn list() -> Vec<InternalPid> {
    bifs::process_list()
  }

  /// Sleeps the current process for the given `timeout`.
  ///
  /// REF: **N/A**
  pub async fn sleep(timeout: Duration) {
    time::sleep(timeout).await
  }

  /// Yields execution back to the runtime.
  ///
  /// REF: **N/A**
  pub async fn yield_now() {
    task::yield_now().await;
  }

  /// Sends an exit signal with the given `reason` to `pid`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#exit/2>
  pub fn exit(pid: impl ProcessId, reason: impl Into<ExitReason>) {
    todo!("exit/2")
  }

  /// Returns `true` if the process exists and is alive, that is, is not exiting
  /// and has not exited. Otherwise returns `false`.
  pub fn alive(pid: InternalPid) -> bool {
    todo!("alive/1")
  }

  /// Returns the process flags of the calling process.
  ///
  /// REF: **N/A**
  pub fn get_flags() -> ProcessFlags {
    Self::with(|this| bifs::process_get_flags(this))
  }

  /// Sets the process flags of the calling process.
  ///
  /// REF: **N/A**
  pub fn set_flags(flags: ProcessFlags) {
    Self::with(|this| bifs::process_set_flags(this, flags))
  }

  /// Sets the process flag indicated to the specified value.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#process_flag/2>
  pub fn set_flag(flag: ProcessFlags, value: bool) {
    Self::with(|this| bifs::process_set_flag(this, flag, value))
  }

  /// Returns information about the process identified by `pid`.
  ///
  /// Returns `None` if the process is not alive.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#process_info/1>
  pub fn info(pid: InternalPid) -> Option<ProcessInfo> {
    Self::with(|this| bifs::process_info(this, pid))
  }

  // ---------------------------------------------------------------------------
  // General API - Spawning & Messaging
  // ---------------------------------------------------------------------------

  /// Spawns a new process to handle `future`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#spawn/1>
  pub fn spawn<F>(future: F) -> InternalPid
  where
    F: Future<Output = ()> + Send + 'static,
  {
    let opts: SpawnConfig = SpawnConfig::new();
    let data: SpawnHandle = Self::spawn_opt(future, opts);

    match data {
      SpawnHandle::Process(pid) => pid,
      SpawnHandle::Monitor(_, _) => unreachable!(),
    }
  }

  /// Spawns a new atomically linked process to handle `future`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#spawn_link/1>
  pub fn spawn_link<F>(future: F) -> InternalPid
  where
    F: Future<Output = ()> + Send + 'static,
  {
    let opts: SpawnConfig = SpawnConfig::new_link();
    let data: SpawnHandle = Self::spawn_opt(future, opts);

    match data {
      SpawnHandle::Process(pid) => pid,
      SpawnHandle::Monitor(_, _) => unreachable!(),
    }
  }

  /// Spawns a new atomically monitored process to handle `future`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#spawn_monitor/1>
  pub fn spawn_monitor<F>(future: F) -> (InternalPid, MonitorRef)
  where
    F: Future<Output = ()> + Send + 'static,
  {
    let opts: SpawnConfig = SpawnConfig::new_monitor();
    let data: SpawnHandle = Self::spawn_opt(future, opts);

    match data {
      SpawnHandle::Process(_) => unreachable!(),
      SpawnHandle::Monitor(process, monitor) => (process, monitor),
    }
  }

  /// Spawns a new process with the given `options` to handle `future`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#spawn_opt/2>
  pub fn spawn_opt<F>(future: F, options: SpawnConfig) -> SpawnHandle
  where
    F: Future<Output = ()> + Send + 'static,
  {
    Self::with(|this| bifs::process_spawn(this, options, future))
  }

  /// Sends `message` to the given `destination`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#send/2>
  ///
  /// # Errors
  ///
  /// Raises [`Exception`] if the destination is an unregistered name.
  ///
  /// [`Exception`]: crate::core::Exception
  pub fn send<M>(dest: impl Into<ExternalDest>, term: M)
  where
    M: Item,
  {
    Self::with(|this| bifs::process_send(this, dest.into(), Term::new(term)))
  }

  /// Checks if there is a message matching the given type `T` in the mailbox of
  /// the current process.
  ///
  /// REF: <https://www.erlang.org/doc/system/expressions.html#receive>
  pub async fn receive<T>() -> Message<Box<T>>
  where
    T: 'static,
  {
    Self::with(|this| bifs::process_receive::<T>(this)).await
  }

  /// Checks if there is a message matching the given type `T` in the mailbox of
  /// the current process.
  ///
  /// REF: <https://www.erlang.org/doc/system/expressions.html#receive>
  pub async fn receive_exact<T>() -> Box<T>
  where
    T: 'static,
  {
    Self::with(|this| bifs::process_receive_exact::<T>(this)).await
  }

  /// Checks if there is a message in the mailbox of the current process.
  ///
  /// REF: <https://www.erlang.org/doc/system/expressions.html#receive>
  pub async fn receive_any() -> DynMessage {
    Self::with(|this| bifs::process_poll(this.mpid, |_| true)).await
  }

  // ---------------------------------------------------------------------------
  // General API - Links, Monitors, Aliases
  // ---------------------------------------------------------------------------

  /// Creates a link between the calling process and the given `pid`.
  ///
  /// Links are bidirectional. Linked processes can be unlinked by using [`Process::unlink`].
  ///
  /// If such a link exists already, this function does nothing since there can
  /// only be one link between two given processes. If a process tries to create
  /// a link to itself, nothing will happen.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#link/1>
  pub fn link(pid: impl ProcessId) {
    todo!("link/1")
  }

  /// Removes the link between the calling process and the given `pid`.
  ///
  /// If there is no such link, this function does nothing. If `pid` does not
  /// exist, this function does not produce any errors and simply does nothing.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#unlink/1>
  pub fn unlink(pid: impl ProcessId) {
    todo!("unlink/1")
  }

  /// Starts monitoring the given `item` from the calling process.
  ///
  /// Once the monitored process dies, a message is delivered to the monitoring
  /// process in the shape of:
  ///
  /// ```text
  /// {:DOWN, ref, :process, object, reason}
  /// ```
  ///
  /// where:
  ///   - `ref` is a monitor reference returned by this function;
  ///   - `object` is either a `pid` of the monitored process (if monitoring a
  ///     PID) or `{name, node}` (if monitoring a remote or local name);
  ///   - `reason` is the exit reason.
  ///
  /// If the process is already dead when calling [`Process::monitor`], a `:DOWN`
  /// message is delivered immediately.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#monitor/2>
  pub fn monitor(item: impl Into<ExternalDest>) -> MonitorRef {
    todo!("monitor/1")
  }

  /// Demonitors the monitor identified by the given `reference`.
  ///
  /// If `reference` is a reference which the calling process obtained by
  /// calling [`Process::monitor`], that monitoring is turned off. If the
  /// monitoring is already turned off, nothing happens.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#demonitor/1>
  pub fn demonitor(reference: MonitorRef) {
    todo!("demonitor/1")
  }

  /// Creates a process alias.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#alias/0>
  pub fn alias(reply: bool) -> AliasRef {
    todo!("alias/1")
  }

  /// Explicitly deactivates a process alias.
  ///
  /// Returns `true` if `alias` was a currently-active alias for current
  /// processes, or `false` otherwise.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#unalias/1>
  pub fn unalias(alias: AliasRef) -> bool {
    todo!("unalias/1")
  }

  // ---------------------------------------------------------------------------
  // General API - Timers
  // ---------------------------------------------------------------------------

  /// Sends `message` to given `destination` after `time` delay.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#send_after/3>
  pub fn send_after<T>(destination: impl Into<InternalDest>, message: T, time: Duration) -> TimerRef
  where
    T: Send + 'static,
  {
    todo!("send_after/3")
  }

  /// Cancels a timer returned by [`Process::send_after`].
  ///
  /// Returns the duration left until the timer will expire, or `None` if the
  /// timer has already expired/been canceled.
  ///
  /// Even if the timer had expired and the message was sent, this function does
  /// not tell you if the timeout message has arrived at its destination yet.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#cancel_timer/1>
  pub fn cancel_timer(timer: TimerRef) -> Option<Duration> {
    todo!("cancel_timer/1")
  }

  /// Reads a timer created by [`Process::send_after`].
  ///
  /// Returns the duration left until the timer will expire, or `None` if the
  /// timer has already expired.
  ///
  /// Even if the timer had expired and the message was sent, this function does
  /// not tell you if the timeout message has arrived at its destination yet.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#read_timer/1>
  pub fn read_timer(reference: TimerRef) -> Option<Duration> {
    todo!("read_timer/1")
  }

  // ---------------------------------------------------------------------------
  // General API - Local Name Registration
  // ---------------------------------------------------------------------------

  /// Registers the given `pid` under the given `name` on the local node.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#register/2>
  ///
  /// # Errors
  ///
  /// Raises [`Exception`] in the following cases:
  ///
  /// - The PID is not alive
  /// - The PID is currently registered under a different name
  /// - The name is already registered to another PID
  ///
  /// [`Exception`]: crate::core::Exception
  pub fn register(pid: InternalPid, name: impl Into<Atom>) {
    Self::with(|this| bifs::process_register(this, pid, name.into()))
  }

  /// Removes the registered `name`, associated with a PID.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#unregister/1>
  ///
  /// # Errors
  ///
  /// Raises [`Exception`] if the name is not registered to any PID.
  ///
  /// [`Exception`]: crate::core::Exception
  pub fn unregister(name: impl Into<Atom>) {
    Self::with(|this| bifs::process_unregister(this, name.into()))
  }

  /// Returns the PID under `name`, or `None` if the name is not registered.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#whereis/1>
  pub fn whereis(name: impl Into<Atom>) -> Option<InternalPid> {
    bifs::process_whereis(name.into())
  }

  /// Returns a list of names which have been registered using [`Process::register`].
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#registered/0>
  pub fn registered() -> Vec<Atom> {
    bifs::process_registered()
  }

  // ---------------------------------------------------------------------------
  // General API - Process Dictionary
  // ---------------------------------------------------------------------------

  /// Stores the given `key`-`value` pair in the process dictionary.
  ///
  /// The return value of this function is the value that was previously stored
  /// under key, or `None` in case no value was stored under it.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#put/2>
  pub fn put(key: impl Into<Atom>, value: impl Into<Term>) -> Option<Term> {
    Self::with(|this| bifs::process_dict_put(this, key.into(), value.into()))
  }

  /// Returns the value for the given `key` in the process dictionary,
  /// or `None` if `key` is not set.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#get/1>
  pub fn get(key: impl Into<Atom>) -> Option<Term> {
    Self::with(|this| bifs::process_dict_get(this, key.into()))
  }

  /// Deletes the given `key` from the process dictionary.
  ///
  /// Returns the value that was under `key` in the process dictionary,
  /// or `None` if `key` was not stored in the process dictionary.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#erase/1>
  pub fn delete(key: impl Into<Atom>) -> Option<Term> {
    Self::with(|this| bifs::process_dict_delete(this, key.into()))
  }

  /// Clears the prcoess dictionary and returns the previous key-value pairs.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#erase/0>
  pub fn clear() -> Vec<(Atom, Term)> {
    Self::with(|this| bifs::process_dict_clear(this))
  }

  /// Returns a list of all key-value pairs in the process dictionary.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#get/0>
  pub fn pairs() -> Vec<(Atom, Term)> {
    Self::with(|this| bifs::process_dict_pairs(this))
  }

  /// Returns a list of all keys in the process dictionary.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#get_keys/0>
  pub fn keys() -> Vec<Atom> {
    Self::with(|this| bifs::process_dict_keys(this))
  }

  /// Returns a list of all values in the process dictionary.
  ///
  /// REF: **N/A**
  pub fn values() -> Vec<Term> {
    Self::with(|this| bifs::process_dict_values(this))
  }
}
