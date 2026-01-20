use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;

/// A guard proving exclusive access to task-local state.
///
/// This type uses a branded lifetime `'task` to ensure it can only be
/// created from the owning `ProcTask` and cannot escape the task context.
///
/// # Safety Invariant
///
/// A `TaskGuard` can only exist within a `Process::with()` callback,
/// which guarantees:
///
/// 1. We're executing in the task-local context
/// 2. No other code is accessing the guarded data
/// 3. The guard cannot escape the callback scope
pub struct TaskGuard<'task, T> {
  /// Pointer to the guarded data.
  protect: &'task UnsafeCell<T>,
  /// Invariant lifetime marker.
  ///
  /// The `*mut &'task ()` type makes the lifetime invariant,
  /// preventing subtyping that could allow the guard to escape.
  /// The raw pointer makes the guard !Send and !Sync.
  phantom: PhantomData<*mut &'task ()>,
}

impl<'task, T> TaskGuard<'task, T> {
  /// Creates a new guard.
  ///
  /// # Safety
  ///
  /// This must only be called from `ProcTask::internal()` within
  /// a task-local context established by `Process::with()`.
  ///
  /// The caller must guarantee:
  ///
  /// 1. Exclusive access to the data (no other guards exist)
  /// 2. The data is valid for the lifetime `'task`
  /// 3. The guard will not be sent to another thread
  #[inline]
  pub(crate) unsafe fn new(protect: &'task UnsafeCell<T>) -> Self {
    Self::assert_task();

    Self {
      protect,
      phantom: PhantomData,
    }
  }

  /// Returns a reference to the guarded data.
  #[inline]
  pub(crate) fn get(&self) -> &T {
    // SAFETY: The guard's existence proves exclusive access within
    // the task-local context. The lifetime bounds prevent concurrent
    // access from other tasks.
    unsafe { &*self.protect.get() }
  }

  /// Returns a mutable reference to the guarded data.
  #[inline]
  pub(crate) fn get_mut(&mut self) -> &mut T {
    // SAFETY: The guard's existence proves exclusive access within
    // the task-local context. The &mut self ensures no other guards
    // can exist simultaneously (enforced by the borrow checker).
    unsafe { &mut *self.protect.get() }
  }

  #[cfg(not(debug_assertions))]
  fn assert_task() {
    // do nothing
  }

  #[cfg(debug_assertions)]
  fn assert_task() {
    use crate::erts::Process;

    if !Process::is_task() {
      panic!(
        "TaskGuard::new called outside task-local context! \
  	    TaskGuard can only be created within Process::with() callbacks."
      );
    }
  }
}

impl<'task, T> Deref for TaskGuard<'task, T> {
  type Target = T;

  #[inline]
  fn deref(&self) -> &Self::Target {
    self.get()
  }
}

impl<'task, T> DerefMut for TaskGuard<'task, T> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.get_mut()
  }
}

impl<'task, T> Debug for TaskGuard<'task, T>
where
  T: Debug,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    Debug::fmt(self.get(), f)
  }
}
