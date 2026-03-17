use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result;
use std::marker::PhantomData;
use std::ops::Deref;
use std::ops::DerefMut;

/// A guard proving exclusive access to task-local state.
pub struct TaskGuard<'task, T> {
  context: &'task UnsafeCell<T>,
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
  pub(crate) unsafe fn new(context: &'task UnsafeCell<T>) -> Self {
    Self {
      context,
      phantom: PhantomData,
    }
  }

  /// Returns a reference to the guarded data.
  #[inline]
  pub(crate) const fn get(&self) -> &T {
    // SAFETY: The guard's existence proves exclusive access within
    // the task-local context. The lifetime bounds prevent concurrent
    // access from other tasks.
    unsafe { &*self.context.get() }
  }

  /// Returns a mutable reference to the guarded data.
  #[inline]
  pub(crate) const fn get_mut(&mut self) -> &mut T {
    // SAFETY: The guard's existence proves exclusive access within
    // the task-local context. The &mut self ensures no other guards
    // can exist simultaneously.
    unsafe { &mut *self.context.get() }
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
