use std::sync::LazyLock;
use std::time::Duration;
use std::time::SystemTime;

use crate::core::raise;
use crate::lang::Atom;

mod table;

use self::table::AtomTable;
use self::table::AtomTableError;

/// Errai runtime API.
pub struct Runtime;

impl Runtime {
  // ---------------------------------------------------------------------------
  // Exit Codes
  // ---------------------------------------------------------------------------

  /// Execution success.
  pub const E_CODE_SUCCESS: i32 = 0;
  /// Initialization failure.
  pub const E_CODE_FAILURE_INIT: i32 = -1;
  /// Execution failure.
  pub const E_CODE_FAILURE_EXEC: i32 = -2;

  // ---------------------------------------------------------------------------
  // Types
  // ---------------------------------------------------------------------------

  /// Maximum number of characters in an [`Atom`].
  ///
  /// [`Atom`]: crate::lang::Atom
  pub const MAX_ATOM_CHARS: usize = 255;
  /// Maximum number of [`Atom`]s.
  ///
  /// [`Atom`]: crate::lang::Atom
  pub const MAX_ATOM_COUNT: usize = 1 << 20;

  // ---------------------------------------------------------------------------
  // System
  // ---------------------------------------------------------------------------

  /// Default amount of parallelism the tokio runtime should use.
  ///
  /// Note: This value is only used when a default value is not
  ///       retrievable from the host environment.
  pub const DEFAULT_PARALLELISM: usize = 1;
  /// Number of scheduler ticks before polling for external events.
  pub const DEFAULT_EVENT_INTERVAL: u32 = 61;
  /// Number of scheduler ticks before polling the global task queue.
  pub const DEFAULT_GLOBAL_QUEUE_INTERVAL: u32 = 31;
  /// Limit for additional threads spawned by the tokio runtime.
  pub const DEFAULT_MAX_BLOCKING_THREADS: usize = 512;
  /// Maximum number of I/O events processed per scheduler tick.
  pub const DEFAULT_MAX_IO_EVENTS_PER_TICK: usize = 1024;
  /// How long to keep threads in the blocking pool alive.
  pub const DEFAULT_THREAD_KEEP_ALIVE: Duration = Duration::from_millis(10 * 1000);
  /// Stack size (in bytes) for worker threads.
  pub const DEFAULT_THREAD_STACK_SIZE: usize = 2 * 1024 * 1024;

  /// How long to wait for a clean shutdown of the internal runtime.
  pub const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

  /// Number of pre-allocated entries in the process dictionary.
  pub(crate) const CAP_PROC_DICTIONARY: usize = 8;
  // Number of pre-allocated slots in the internal message buffer.
  pub(crate) const CAP_PROC_MSG_BUFFER: usize = 8;

  // Number of pre-allocated process states.
  pub(crate) const CAP_REGISTERED_PROCS: usize = ProcessTable::<ProcessSlot>::DEF_ENTRIES;
  // Number of pre-allocated registered names.
  pub(crate) const CAP_REGISTERED_NAMES: usize = ProcessTable::<ProcessSlot>::MIN_ENTRIES;

  pub(crate) const SPAWN_INIT_LINK: bool = false;
  pub(crate) const SPAWN_INIT_MONITOR: bool = false;
  pub(crate) const SPAWN_INIT_ASYNC_DIST: bool = false;
  pub(crate) const SPAWN_INIT_TRAP_EXIT: bool = false;

  // ---------------------------------------------------------------------------
  // Runtime API
  // ---------------------------------------------------------------------------

  /// Forcefully stops the Errai runtime system.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/erlang.html#halt/2>
  pub fn halt(_status: u8) {
    todo!()
  }

  /// Gracefully stops the Errai runtime system.
  ///
  /// REF: <https://www.erlang.org/doc/apps/erts/init.html#stop/1>
  pub fn stop(_status: u8) {
    todo!()
  }

  /// Returns the current OS system time as a POSIX time duration.
  ///
  /// REF: <https://www.erlang.org/doc/apps/kernel/os.html#system_time/0>
  pub fn time() -> Duration {
    SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap_or(Duration::ZERO)
  }
}

// -----------------------------------------------------------------------------
// Atom Table
// -----------------------------------------------------------------------------

impl Runtime {
  pub(crate) fn get_atom(slot: u32) -> &'static str {
    let Some(term) = ATOM_TABLE.get(slot) else {
      raise!(Error, SysInv, "atom not found");
    };

    term
  }

  pub(crate) fn set_atom(term: &str) -> u32 {
    match ATOM_TABLE.set(term) {
      Ok(slot) => slot,
      Err(AtomTableError::AtomTooLarge) => {
        raise!(Error, SysCap, "atom too large");
      }
      Err(AtomTableError::TooManyAtoms) => {
        raise!(Error, SysCap, "too many atoms");
      }
    }
  }
}

static ATOM_TABLE: LazyLock<AtomTable> = LazyLock::new(|| {
  let table: AtomTable = AtomTable::new();

  assert_eq!(table.set("").unwrap(), Atom::EMPTY.into_slot());

  assert_eq!(table.set("kill").unwrap(), Atom::KILL.into_slot());
  assert_eq!(table.set("killed").unwrap(), Atom::KILLED.into_slot());
  assert_eq!(table.set("normal").unwrap(), Atom::NORMAL.into_slot());

  assert_eq!(table.set("noproc").unwrap(), Atom::NOPROC.into_slot());
  assert_eq!(table.set("noconn").unwrap(), Atom::NOCONN.into_slot());

  table
});
