use std::marker::PhantomData;

use crate::core::ProcTable;

/// A marker representing the right to claim one slot in the table.
#[repr(transparent)]
pub(crate) struct Permit<'table, T> {
  marker: PhantomData<&'table ProcTable<T>>,
}

impl<'table, T> Permit<'table, T> {
  #[inline]
  pub(crate) const fn new(_table: &'table ProcTable<T>) -> Self {
    Self {
      marker: PhantomData,
    }
  }
}
