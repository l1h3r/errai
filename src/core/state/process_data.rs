use bitflags::bitflags;
use hashbrown::HashMap;
use parking_lot::Mutex;
use parking_lot::RwLock;
use std::num::NonZeroU64;
use std::ops::Deref;
use std::sync::OnceLock;
use tokio::task::JoinHandle;
use triomphe::Arc;

use crate::bifs;
use crate::core::ProcDict;
use crate::core::ProcLink;
use crate::core::ProcMail;
use crate::core::ProcSend;
use crate::lang::Atom;
use crate::lang::Exit;
use crate::lang::InternalPid;

// -----------------------------------------------------------------------------
// Process Flags
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
// Process Info
// -----------------------------------------------------------------------------

/// Information about a process.
#[derive(Debug)]
pub struct ProcessInfo {}

// -----------------------------------------------------------------------------
// Proc Task Handle
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct ProcTask {
  pub(crate) inner: Arc<ProcData>,
}

impl Drop for ProcTask {
  fn drop(&mut self) {
    bifs::process_delete(self);
  }
}

impl Deref for ProcTask {
  type Target = ProcData;

  #[inline]
  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}

// -----------------------------------------------------------------------------
// Proc Data
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[cfg_attr(target_pointer_width = "32", repr(C, align(4)))]
#[cfg_attr(target_pointer_width = "64", repr(C, align(8)))]
pub(crate) struct ProcData {
  pub(crate) readonly: ProcReadOnly,
  pub(crate) internal: Mutex<ProcInternal>,
  pub(crate) external: RwLock<ProcExternal>,
}

impl Drop for ProcData {
  fn drop(&mut self) {
    if let Some(root) = self.readonly.root {
      tracing::trace!(pid = %self.readonly.mpid, parent = %root, "Process drop");
    } else {
      tracing::trace!(pid = %self.readonly.mpid, "Process drop");
    }
  }
}

// -----------------------------------------------------------------------------
// Proc Read-only
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcReadOnly {
  /// PID of the process.
  pub(crate) mpid: InternalPid,
  /// Sending side of the process signal queue.
  pub(crate) send: ProcSend,
  /// Process that spawned this one.
  pub(crate) root: Option<InternalPid>,
  /// Handle to the internal process task.
  pub(crate) task: OnceLock<JoinHandle<()>>,
}

impl ProcReadOnly {
  /// Create a new `ProcReadOnly`.
  ///
  /// Note: `mpid` is set to invalid value.
  #[inline]
  pub(crate) fn new(send: ProcSend, root: Option<InternalPid>) -> Self {
    Self {
      mpid: InternalPid::from_bits(0),
      send,
      root,
      task: OnceLock::new(),
    }
  }
}

// -----------------------------------------------------------------------------
// Proc Internal
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcInternal {
  /// Process flags.
  pub(crate) flags: ProcessFlags,
  /// Internal message queue.
  pub(crate) inbox: ProcMail,
  /// Process-unique identifier.
  pub(crate) puid: NonZeroU64,
  /// Linked process information.
  pub(crate) links: HashMap<InternalPid, ProcLink>,
  /// The process dictionary.
  pub(crate) dictionary: ProcDict,
  /// Process I/O leader.
  pub(crate) group_leader: InternalPid,
}

impl ProcInternal {
  /// SAFETY: The value is `1`.
  pub(crate) const NZ_ONE: NonZeroU64 = unsafe { NonZeroU64::new_unchecked(1) };

  /// Create a new `ProcInternal`.
  ///
  /// Note: `group_leader` is set to invalid value.
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      flags: ProcessFlags::empty(),
      inbox: ProcMail::new(),
      links: HashMap::new(),
      puid: Self::NZ_ONE,
      dictionary: ProcDict::new(),
      group_leader: InternalPid::from_bits(0),
    }
  }

  /// Returns the next process-unique identifier.
  #[inline]
  pub(crate) fn next_puid(&mut self) -> NonZeroU64 {
    let next: NonZeroU64 = self.puid;

    self.puid = match self.puid.checked_add(1) {
      Some(value) => value,
      None => Self::NZ_ONE,
    };

    next
  }
}

// -----------------------------------------------------------------------------
// Proc External
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[repr(C)]
pub(crate) struct ProcExternal {
  /// Registered name.
  pub(crate) name: Option<Atom>,
  /// Reason for termination.
  pub(crate) exit: Option<Exit>,
}

impl ProcExternal {
  /// Create a new `ProcExternal`.
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      name: None,
      exit: None,
    }
  }
}
