#[cfg(not(loom))]
pub(crate) mod export {
  pub(crate) mod alloc {
    pub(crate) use std::alloc::Layout;
    pub(crate) use std::alloc::alloc;
    pub(crate) use std::alloc::dealloc;
  }

  pub(crate) mod hint {
    pub(crate) use std::hint::spin_loop;
  }

  pub(crate) mod sync {
    pub(crate) use parking_lot::Mutex;
    pub(crate) use parking_lot::RwLock;
    pub(crate) use std::sync::Barrier;

    pub(crate) mod atomic {
      pub(crate) use std::sync::atomic::AtomicPtr;
      pub(crate) use std::sync::atomic::AtomicU32;
      pub(crate) use std::sync::atomic::AtomicU64;
      pub(crate) use std::sync::atomic::AtomicUsize;
      pub(crate) use std::sync::atomic::Ordering;
    }
  }
}

#[cfg(loom)]
pub(crate) mod export {
  pub(crate) mod alloc {
    pub(crate) use loom::alloc::Layout;
    pub(crate) use loom::alloc::alloc;
    pub(crate) use loom::alloc::dealloc;
  }

  pub(crate) mod hint {
    pub(crate) use loom::hint::spin_loop;
  }

  pub(crate) mod sync {
    pub(crate) use loom::sync::Barrier;
    pub(crate) use loom::sync::Mutex;
    pub(crate) use loom::sync::RwLock;

    pub(crate) mod atomic {
      pub(crate) use loom::sync::atomic::AtomicPtr;
      pub(crate) use loom::sync::atomic::AtomicU32;
      pub(crate) use loom::sync::atomic::AtomicU64;
      pub(crate) use loom::sync::atomic::AtomicUsize;
      pub(crate) use loom::sync::atomic::Ordering;
    }
  }
}

#[doc(inline)]
pub(crate) use self::export::*;
