use std::ops::Deref;
use triomphe::Arc;

use crate::bifs;
use crate::proc::ProcData;

/// Process task wrapper that triggers cleanup on drop.
///
/// This type wraps the process data [`Arc`] and implements [`Drop`] to
/// remove the process from the global process table when the task completes.
///
/// # Drop Behavior
///
/// Calls [`bifs::process_delete()`] to remove the process from the table
/// and propagate exit signals to linked/monitored processes.
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
