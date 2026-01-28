use std::cell::Cell;
use std::num::NonZeroU32;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::thread::AccessError;

use crate::core::fatal;

thread_local! {
  static CURRENT: Cell<Option<ThreadId>> = const { Cell::new(None) };
}

/// A unique identifier for a running thread.
///
/// This has a maximum value of `(2 ^ 18) - 1`.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub(crate) struct ThreadId {
  inner: NonZeroU32,
}

impl ThreadId {
  // The maximum number of permitted worker threads.
  pub(crate) const MAX_THREADS: u32 = 1_u32.strict_shl(18).strict_sub(1);

  /// Returns the unique identifier of the calling thread.
  #[inline]
  pub(crate) fn current() -> Result<Self, AccessError> {
    CURRENT.try_with(|thread| {
      thread.get().unwrap_or_else(
        #[cold]
        || {
          let id: ThreadId = next_thread_id();
          thread.set(Some(id));
          id
        },
      )
    })
  }

  /// Returns this `ThreadId` as a numeric identifier.
  #[inline]
  pub(crate) fn as_u32(&self) -> NonZeroU32 {
    self.inner
  }
}

fn next_thread_id() -> ThreadId {
  static ID: AtomicU32 = AtomicU32::new(0);

  let mut last: u32 = ID.load(Ordering::Relaxed);

  'next: loop {
    let Some(id) = last.checked_add(1) else {
      exhausted();
    };

    if id > ThreadId::MAX_THREADS {
      exhausted();
    }

    match ID.compare_exchange_weak(last, id, Ordering::Relaxed, Ordering::Relaxed) {
      Ok(_) => {
        break 'next ThreadId {
          // SAFETY: `id` is derived from `last + 1` and `last` never wraps.
          inner: unsafe { NonZeroU32::new_unchecked(id) },
        };
      }
      Err(next) => last = next,
    }
  }
}

#[cold]
fn exhausted() -> ! {
  fatal!("failed to generate unique thread ID: bitspace exhausted")
}
