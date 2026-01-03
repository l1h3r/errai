use bitflags::bitflags;
use std::collections::HashMap;
use std::time::Duration;

use crate::erts::Message;
use crate::lang::Atom;
use crate::lang::DynPid;
use crate::lang::ExitReason;
use crate::lang::ExternalDest;
use crate::lang::InternalDest;
use crate::lang::InternalPid;
use crate::lang::InternalRef;
use crate::lang::RawPid;
use crate::lang::Term;

// -----------------------------------------------------------------------------
// @alias - References
// -----------------------------------------------------------------------------

pub type AliasRef = InternalRef;
pub type MonitorRef = InternalRef;
pub type TimerRef = InternalRef;

// -----------------------------------------------------------------------------
// @trait - ProcessId
// -----------------------------------------------------------------------------

pub trait ProcessId {
  /// Returns the raw PID bits.
  fn bits(&self) -> RawPid;

  /// Returns the name of the node that spawned this PID,
  /// or `None` if the PID is an internal PID.
  fn node(&self) -> Option<Atom>;

  /// Converts `self` into a dynamic PID.
  fn into_dyn(self) -> DynPid;

  /// Returns `true` if the PID is internal (created on this node).
  #[inline]
  fn is_internal(&self) -> bool {
    self.node().is_none()
  }

  /// Returns `true` if the PID is external (created on another node).
  #[inline]
  fn is_external(&self) -> bool {
    self.node().is_some()
  }
}

// -----------------------------------------------------------------------------
// @type - ProcessFlags
// -----------------------------------------------------------------------------

bitflags! {
  #[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
  pub struct ProcessFlags: u32 {
    const TRAP_EXIT = 1 << 1;
  }
}

// -----------------------------------------------------------------------------
// @type - ProcessInfo
// -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct ProcessInfo {
  pub dictionary: HashMap<Atom, Term>,
}

// -----------------------------------------------------------------------------
// @api - Process
// -----------------------------------------------------------------------------

/// Errai process API.
pub struct Process;

impl Process {
  // ---------------------------------------------------------------------------
  // General API
  // ---------------------------------------------------------------------------

  /// Returns the process identifier of the calling process.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#self/0>
  pub fn this() -> InternalPid {
    todo!("this/0")
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
    todo!("list/0")
  }

  /// Sleeps the current process for the given `timeout`.
  ///
  /// REF: **N/A**
  pub fn sleep(timeout: Duration) {
    todo!("sleep/1")
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

  /// Sets the process flag indicated to the specified value. Returns the
  /// previous value of the flag.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#process_flag/2>
  pub fn set_flag(flag: ProcessFlags, value: bool) -> bool {
    todo!("set_flag/2")
  }

  /// Returns the process flags of the calling process.
  ///
  /// REF: **N/A**
  pub fn get_flags() -> ProcessFlags {
    todo!("get_flags/0")
  }

  /// Sets the process flags of the calling process.
  ///
  /// REF: **N/A**
  pub fn set_flags(flags: ProcessFlags) {
    todo!("set_flags/1")
  }

  /// Returns information about the process identified by `pid`.
  ///
  /// Returns `None` if the process is not alive.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#process_info/1>
  pub fn info(pid: InternalPid) -> Option<ProcessInfo> {
    todo!("info/1")
  }

  // ---------------------------------------------------------------------------
  // General API - Spawning & Messaging
  // ---------------------------------------------------------------------------

  /// Spawns a new process to handle `future`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#spawn/1>
  pub fn spawn<T>(future: T) -> InternalPid
  where
    T: Future<Output = ()> + Send + 'static,
  {
    todo!("spawn/1")
  }

  /// Spawns a new atomically linked process to handle `future`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#spawn_link/1>
  pub fn spawn_link<T>(future: T) -> InternalPid
  where
    T: Future<Output = ()> + Send + 'static,
  {
    todo!("spawn_link/1")
  }

  /// Spawns a new atomically monitored process to handle `future`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#spawn_monitor/1>
  pub fn spawn_monitor<T>(future: T) -> InternalPid
  where
    T: Future<Output = ()> + Send + 'static,
  {
    todo!("spawn_monitor/1")
  }

  /// Sends `message` to the given `destination`.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#send/2>
  ///
  /// # Panics
  ///
  /// Panics with [`ArgumentError`] if the destination is an unregistered name.
  pub fn send<T>(destination: impl Into<ExternalDest>, message: T)
  where
    T: Send + 'static,
  {
    todo!("send/2")
  }

  /// Sends `message` to given `destination` after `time` delay.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#send_after/3>
  pub fn send_after<T>(destination: impl Into<InternalDest>, message: T, time: Duration) -> TimerRef
  where
    T: Send + 'static,
  {
    todo!("send_after/3")
  }

  /// Checks if there is a message matching the given type `T` in the mailbox of
  /// the current process.
  ///
  /// REF: <https://www.erlang.org/doc/system/expressions.html#receive>
  pub async fn receive<T>() -> Message<T>
  where
    T: 'static,
  {
    todo!("receive/0")
  }

  // ---------------------------------------------------------------------------
  // General API - Links & Monitors
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
  /// ```
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

  // ---------------------------------------------------------------------------
  // General API - Timers
  // ---------------------------------------------------------------------------

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
  // General API - Aliases
  // ---------------------------------------------------------------------------

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
  // General API - Local Name Registration
  // ---------------------------------------------------------------------------

  /// Registers the given `pid` under the given `name` on the local node.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#register/2>
  ///
  /// # Panics
  ///
  /// Panics with [`ArgumentError`] in the following cases:
  ///
  /// - The PID is not alive
  /// - The PID is currently registered under a different name
  /// - The name is already registered to another PID
  pub fn register(pid: InternalPid, name: impl Into<Atom>) {
    todo!("register/2")
  }

  /// Removes the registered `name`, associated with a PID.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#unregister/1>
  ///
  /// # Panics
  ///
  /// Panics with [`ArgumentError`] if the name is not registered to any PID.
  pub fn unregister(name: impl Into<Atom>) {
    todo!("unregister/1")
  }

  /// Returns the PID under `name`, or `None` if the name is not registered.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#whereis/1>
  pub fn whereis(name: impl Into<Atom>) -> Option<InternalPid> {
    todo!("whereis/1")
  }

  /// Returns a list of names which have been registered using [`Process::register`].
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#registered/0>
  pub fn registered() -> Vec<Atom> {
    todo!("registered/0")
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
    todo!("put/2")
  }

  /// Returns the value for the given `key` in the process dictionary,
  /// or `None` if `key` is not set.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#get/1>
  pub fn get(key: impl Into<Atom>) -> Option<Term> {
    todo!("get/1")
  }

  /// Deletes the given `key` from the process dictionary.
  ///
  /// Returns the value that was under `key` in the process dictionary,
  /// or `None` if `key` was not stored in the process dictionary.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#erase/1>
  pub fn delete(key: impl Into<Atom>) -> Option<Term> {
    todo!("delete/1")
  }

  /// Clears the prcoess dictionary and returns the previous key-value pairs.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#erase/0>
  pub fn clear() -> Vec<(Atom, Term)> {
    todo!("clear/0")
  }

  /// Returns a list of all key-value pairs in the process dictionary.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#get/0>
  pub fn pairs() -> Vec<(Atom, Term)> {
    todo!("pairs/0")
  }

  /// Returns a list of all keys in the process dictionary.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#get_keys/0>
  pub fn keys() -> Vec<Atom> {
    todo!("keys/0")
  }

  /// Returns a list of all values in the process dictionary.
  ///
  /// REF: **N/A**
  pub fn values() -> Vec<Term> {
    todo!("values/0")
  }
}
