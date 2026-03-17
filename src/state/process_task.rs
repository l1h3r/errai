use std::ops::Deref;
use triomphe::Arc;

use crate::state::ProcData;
use crate::state::ProcInternal;
use crate::utils::task::TaskGuard;

/// Process task wrapper that triggers cleanup on drop.
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct ProcTask {
  pub(crate) inner: Arc<ProcData>,
}

impl ProcTask {
  /// Returns a guard providing safe mutable access to internal state.
  #[inline]
  pub(crate) fn internal(&self) -> TaskGuard<'_, ProcInternal> {
    // SAFETY: This method is only available in a task-local context, ensuring:
    //         - We're in the owning task's execution context
    //         - No other code can access `internal` simultaneously
    //         - The guard's lifetime is tied to this method call
    unsafe { TaskGuard::new(&self.inner.internal) }
  }
}

impl Drop for ProcTask {
  fn drop(&mut self) {
    // TODO: Cleanup
  }
}

impl Deref for ProcTask {
  type Target = ProcData;

  #[inline]
  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
