use std::ops::Deref;
use triomphe::Arc;

use crate::bifs;
use crate::proc::ProcData;
use crate::proc::ProcInternal;
use crate::proc::TaskGuard;

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

impl ProcTask {
  /// Returns a guard providing safe mutable access to internal state.
  ///
  /// # Safety
  ///
  /// This access pattern is safe because:
  ///
  /// 1. ProcTask only exists in Process::with() callbacks (post-initialization)
  /// 2. The guard's lifetime is tied to &self
  /// 3. The borrow checker prevents overlapping guards
  ///
  /// # Example
  ///
  /// ```ignore
  /// Process::with(|proc| {
  ///   proc.internal().inbox.push(message);
  /// });
  /// ```
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
    bifs::proc_remove(self);
  }
}

impl Deref for ProcTask {
  type Target = ProcData;

  #[inline]
  fn deref(&self) -> &Self::Target {
    &*self.inner
  }
}
