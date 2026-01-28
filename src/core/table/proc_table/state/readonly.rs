use crossbeam_epoch::Atomic;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicUsize;

use crate::core::LocalPid;
use crate::core::table::proc_table::Index;
use crate::core::table::proc_table::Params;
use crate::core::table::proc_table::Slots;

/// Static table metadata stored in a cache-padded section.
///
/// This data is initialized once and never modified, avoiding cache
/// line bouncing between threads.
#[repr(C)]
pub(crate) struct ReadOnly<T> {
  /// Array of table entries (concrete data).
  pub(crate) concrete_array: Slots<Atomic<T>>,
  /// Array mapping abstract to concrete indices.
  pub(crate) abstract_array: Slots<AtomicUsize>,
  /// Index mapping masks and shifts for PID translation.
  pub(crate) id_mask_entry: usize,
  pub(crate) id_mask_block: usize,
  pub(crate) id_mask_index: usize,
  pub(crate) id_shift_block: u32,
  pub(crate) id_shift_index: u32,
}

impl<T> ReadOnly<T> {
  #[inline]
  pub(crate) fn new(capacity: usize) -> Self {
    let params: Params = Params::new(capacity);
    let concrete_array: Slots<Atomic<T>> = alloc_concrete_array(&params);
    let abstract_array: Slots<AtomicUsize> = alloc_abstract_array(&params);

    Self {
      concrete_array,
      abstract_array,
      id_mask_entry: params.id_mask_entry,
      id_mask_block: params.id_mask_block,
      id_mask_index: params.id_mask_index,
      id_shift_block: params.id_shift_block,
      id_shift_index: params.id_shift_index,
    }
  }

  // ---------------------------------------------------------------------------
  // Index Mapping
  //
  // The following functions are based on the Erlang/OTP Implementation
  //
  // payload_to_abstract  -> erts_ptab_pixdata2data
  // payload_to_concrete  -> erts_ptab_pixdata2pix
  // pid_to_abstract      -> erts_ptab_id2data
  // pid_to_concrete      -> erts_ptab_id2pix
  // abstract_to_concrete -> erts_ptab_data2pix
  // abstract_to_payload  -> erts_ptab_data2pixdata
  // abstract_to_pid      -> erts_ptab_make_id
  // ---------------------------------------------------------------------------

  /// Extracts the abstract sequential index from PID payload bits.
  #[inline]
  pub(crate) const fn payload_to_abstract(&self, payload: usize) -> usize {
    let mut value: usize = payload & !self.id_mask_entry;
    value |= (payload >> self.id_shift_block) & self.id_mask_block;
    value |= (payload & self.id_mask_index) << self.id_shift_index;
    value
  }

  /// Extracts the concrete cache-aware index from PID payload bits.
  #[inline]
  pub(crate) const fn payload_to_concrete(&self, payload: usize) -> Index<'_, T> {
    Index::new(self, payload & self.id_mask_entry)
  }

  /// Extracts abstract sequential index from PID.
  #[inline]
  pub(crate) const fn pid_to_abstract(&self, pid: LocalPid) -> usize {
    self.payload_to_abstract(pid.into_bits() >> LocalPid::TAG_BITS)
  }

  /// Extracts concrete cache-aware index from PID.
  #[inline]
  pub(crate) const fn pid_to_concrete(&self, pid: LocalPid) -> Index<'_, T> {
    self.payload_to_concrete(pid.into_bits() >> LocalPid::TAG_BITS)
  }

  /// Converts abstract sequential index to concrete cache-aware index.
  #[inline]
  pub(crate) const fn abstract_to_concrete(&self, abstract_idx: usize) -> Index<'_, T> {
    let mut value: usize = 0;
    value += (abstract_idx & self.id_mask_block) << self.id_shift_block;
    value += (abstract_idx >> self.id_shift_index) & self.id_mask_index;
    Index::new(self, value)
  }

  /// Converts abstract sequential index to PID payload bits.
  #[inline]
  pub(crate) const fn abstract_to_payload(&self, abstract_idx: usize) -> usize {
    let value: usize = abstract_idx & !self.id_mask_entry;
    let value: usize = value | self.abstract_to_concrete(abstract_idx).get();
    debug_assert!(self.payload_to_abstract(value) == abstract_idx);
    value
  }

  /// Converts abstract sequential index to complete PID.
  #[inline]
  pub(crate) const fn abstract_to_pid(&self, abstract_idx: usize) -> LocalPid {
    let value: usize = self.abstract_to_payload(abstract_idx & LocalPid::PID_MASK);
    let value: usize = (value << LocalPid::TAG_BITS) | LocalPid::TAG_DATA;

    LocalPid::from_bits(value)
  }
}

#[inline]
fn alloc_concrete_array<T>(params: &Params) -> Slots<Atomic<T>> {
  let mut data: Slots<MaybeUninit<Atomic<T>>> = Slots::new_uninit(params.length);

  for item in data.as_mut_slice() {
    item.write(const { Atomic::null() });
  }

  // SAFETY: We just initialized the entire array
  unsafe { data.assume_init() }
}

#[inline]
fn alloc_abstract_array(params: &Params) -> Slots<AtomicUsize> {
  let mut data: Slots<MaybeUninit<AtomicUsize>> = Slots::new_uninit(params.length);

  let slice: &mut [MaybeUninit<AtomicUsize>] = data.as_mut_slice();
  let mut cursor: usize = 0;

  for block in 0..params.blocks.get() {
    for index in 0..Params::SLOT_COUNT {
      let value: usize = index.wrapping_mul(params.blocks.get()).wrapping_add(block);

      slice[cursor].write(AtomicUsize::new(value));
      cursor += 1;
    }
  }

  // SAFETY: We just initialized the entire array
  unsafe { data.assume_init() }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use super::*;

  const MIN_ENTRIES_SHIFT: usize = Params::MIN_ENTRIES_SHIFT;
  const MAX_ENTRIES_SHIFT: usize = 20; // 30s per test otherwise

  #[test]
  fn test_abstract_to_concrete_covers_all_slots() {
    for shift in MIN_ENTRIES_SHIFT..=MAX_ENTRIES_SHIFT {
      let capacity: usize = 1 << shift;
      let readonly: ReadOnly<u64> = ReadOnly::new(capacity);
      let mut used: HashSet<usize> = HashSet::with_capacity(capacity);

      for abstract_idx in 0..capacity {
        used.insert(readonly.abstract_to_concrete(abstract_idx).get());
      }

      // All indices should be covered exactly once
      assert_eq!(used.len(), capacity);
    }
  }

  #[test]
  fn test_abstract_to_pid_has_correct_tag() {
    for shift in MIN_ENTRIES_SHIFT..=MAX_ENTRIES_SHIFT {
      let capacity: usize = 1 << shift;
      let readonly: ReadOnly<u64> = ReadOnly::new(capacity);

      for abstract_idx in 0..capacity {
        let pid: LocalPid = readonly.abstract_to_pid(abstract_idx);
        let raw: usize = pid.into_bits();

        // Check that the tag is correct
        assert_eq!(raw & LocalPid::TAG_MASK, LocalPid::TAG_DATA);
      }
    }
  }

  #[test]
  fn test_abstract_to_payload_roundtrip() {
    for shift in MIN_ENTRIES_SHIFT..=MAX_ENTRIES_SHIFT {
      let capacity: usize = 1 << shift;
      let readonly: ReadOnly<u64> = ReadOnly::new(capacity);

      for abstract_idx in 0..capacity {
        let convert: usize = readonly.abstract_to_payload(abstract_idx);
        let recover: usize = readonly.payload_to_abstract(convert);

        assert_eq!(abstract_idx, recover);
      }
    }
  }

  #[test]
  fn test_abstract_to_pid_roundtrip() {
    for shift in MIN_ENTRIES_SHIFT..=MAX_ENTRIES_SHIFT {
      let capacity: usize = 1 << shift;
      let readonly: ReadOnly<u64> = ReadOnly::new(capacity);

      for abstract_idx in 0..capacity {
        let convert: LocalPid = readonly.abstract_to_pid(abstract_idx);
        let recover: usize = readonly.pid_to_abstract(convert);

        assert_eq!(abstract_idx, recover);
      }
    }
  }

  #[test]
  fn test_pid_to_concrete_matches_direct_conversion() {
    for shift in MIN_ENTRIES_SHIFT..=MAX_ENTRIES_SHIFT {
      let capacity: usize = 1 << shift;
      let readonly: ReadOnly<u64> = ReadOnly::new(capacity);

      for abstract_idx in 0..capacity {
        let convert: LocalPid = readonly.abstract_to_pid(abstract_idx);
        let concrete_idx: usize = readonly.abstract_to_concrete(abstract_idx).get();
        let concrete_pid: usize = readonly.pid_to_concrete(convert).get();

        assert_eq!(concrete_idx, concrete_pid);
      }
    }
  }

  #[test]
  fn test_payload_to_concrete_extracts_entry_correctly() {
    for shift in MIN_ENTRIES_SHIFT..=MAX_ENTRIES_SHIFT {
      let capacity: usize = 1 << shift;
      let readonly: ReadOnly<u64> = ReadOnly::new(capacity);

      for abstract_idx in 0..capacity {
        let payload: usize = readonly.abstract_to_payload(abstract_idx);
        let concrete: usize = readonly.payload_to_concrete(payload).get();

        // The concrete index should match the entry portion of payload
        assert_eq!(concrete, payload & (readonly.id_mask_entry as usize));
      }
    }
  }

  #[test]
  fn test_cache_line_distribution() {
    // Verify that consecutive base indices are distributed across cache lines
    for shift in MIN_ENTRIES_SHIFT..=MAX_ENTRIES_SHIFT {
      let capacity: usize = 1 << shift;
      let readonly: ReadOnly<u64> = ReadOnly::new(capacity);
      let params: Params = Params::new(capacity);

      if params.blocks.get() <= 1 {
        continue; // Skip solo blocks
      }

      // First SLOT_COUNT base indices should map to different cache lines
      let mut blocks_seen: HashSet<usize> = HashSet::with_capacity(Params::SLOT_COUNT);

      for slot in 0..Params::SLOT_COUNT.min(capacity) {
        let index: usize = readonly.abstract_to_concrete(slot).get();
        let block: usize = index / Params::SLOT_COUNT;
        blocks_seen.insert(block);
      }

      // Should spread across multiple blocks
      assert!(blocks_seen.len() > 1 || params.blocks.get() == 1);
    }
  }

  #[test]
  fn test_serial_number_preservation() {
    // When abstract includes a serial component, it should be preserved
    let capacity: usize = 1 << 10;
    let readonly: ReadOnly<u64> = ReadOnly::new(capacity);

    // Simulate multiple generations of the same slot
    for generation in 0..16 {
      let serial: usize = generation * capacity;

      for slot in 0..capacity {
        let abstract_idx: usize = serial + slot;
        let convert: LocalPid = readonly.abstract_to_pid(abstract_idx);
        let recover: usize = readonly.pid_to_abstract(convert);

        // recover should match (within PID_BITS)
        assert_eq!(abstract_idx & LocalPid::PID_MASK, recover);
      }
    }
  }
}
