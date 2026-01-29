use std::marker::PhantomData;

use crate::core::table::proc_table::ReadOnly;

#[repr(transparent)]
pub(crate) struct Index<'table, T> {
  source: usize,
  marker: PhantomData<&'table ReadOnly<T>>,
}

impl<'table, T> Index<'table, T> {
  #[inline]
  pub(crate) const fn new(_table: &'table ReadOnly<T>, source: usize) -> Self {
    Self {
      source,
      marker: PhantomData,
    }
  }

  #[inline]
  pub(crate) const fn get(self) -> usize {
    self.source
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_roundtrip() {
    let table: ReadOnly<u64> = ReadOnly::new(1 << 4);
    let index: Index<'_, u64> = Index::new(&table, 123);

    assert_eq!(index.get(), 123);
  }
}
