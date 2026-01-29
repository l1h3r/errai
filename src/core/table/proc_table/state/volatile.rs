use std::num::NonZeroUsize;
use std::sync::atomic::AtomicU32;

use crate::core::table::proc_table::Params;
use crate::core::table::proc_table::ReadOnly;

/// Frequently modified table state stored in a cache-padded section.
#[repr(C)]
pub(crate) struct Volatile {
  /// The total number of entries currently in the table.
  pub(crate) len: AtomicU32,
  /// Allocation counter for the next slot to allocate.
  pub(crate) aid: AtomicU32,
  /// Free counter for the next slot to return to the free list.
  pub(crate) fid: AtomicU32,
}

impl Volatile {
  #[inline]
  pub(crate) fn new<T>(readonly: &ReadOnly<T>) -> Self {
    // The table can only yield `Params::MAX_ENTRIES - 1` unique identifiers.
    //
    // To fix this without spreading complexity throughout the implementation,
    // we increment the table `len` field to permanently reserve one slot.
    //
    // The end result is a table that wastes one slot but simplifies the code.
    let cap: NonZeroUsize = readonly.concrete_array.len();
    let len: u32 = u32::from(cap == Params::MAX_ENTRIES);

    Self {
      len: AtomicU32::new(len),
      aid: AtomicU32::new(0),
      fid: AtomicU32::new(0),
    }
  }
}
