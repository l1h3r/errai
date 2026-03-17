use std::ops::Deref;
use triomphe::Arc;

use crate::erts::ProcData;
use crate::erts::ProcInternal;

/// Process task wrapper that triggers cleanup on drop.
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct ProcTask {
  pub(crate) inner: Arc<ProcData>,
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
