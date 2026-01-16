use std::ops::Deref;
use triomphe::Arc;

use crate::bifs;
use crate::proc::ProcData;

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
