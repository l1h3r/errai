use crossbeam_epoch::Atomic;
use crossbeam_utils::CachePadded;
use std::num::NonZeroUsize;
use std::sync::atomic::AtomicUsize;

#[repr(C)]
pub(crate) struct Params {
  pub(crate) blocks: NonZeroUsize,
  pub(crate) length: NonZeroUsize,
  pub(crate) id_mask_entry: usize,
  pub(crate) id_mask_block: usize,
  pub(crate) id_mask_index: usize,
  pub(crate) id_shift_block: u32,
  pub(crate) id_shift_index: u32,
}

impl Params {
  /// The assumed size of a cache line in bytes.
  pub(crate) const CACHE_LINE: usize = size_of::<CachePadded<u8>>();

  /// The number of bytes required to store a table entry.
  pub(crate) const ENTRY_SIZE: usize = {
    assert!(
      align_of::<AtomicUsize>() == align_of::<Atomic<()>>(),
      "atomic pointer != atomic usize",
    );

    assert!(
      size_of::<AtomicUsize>() == size_of::<Atomic<()>>(),
      "atomic pointer != atomic usize",
    );

    size_of::<Atomic<()>>()
  };

  /// The total number of table slots available in a cache line.
  pub(crate) const SLOT_COUNT: usize = {
    assert!(
      Self::CACHE_LINE % Self::ENTRY_SIZE == 0,
      "cache line must be divisible by entry size",
    );

    let count: usize = Self::CACHE_LINE / Self::ENTRY_SIZE;

    assert!(count.is_power_of_two(), "slot count must be a power of two");

    count
  };

  /// Shift used to compute `MIN_ENTRIES`.
  pub(crate) const MIN_ENTRIES_SHIFT: usize = 4;
  /// Minimum number of entries supported in the table.
  pub(crate) const MIN_ENTRIES: NonZeroUsize = assert_nonzero(1 << Self::MIN_ENTRIES_SHIFT);

  /// Shift used to compute `MAX_ENTRIES`.
  pub(crate) const MAX_ENTRIES_SHIFT: usize = 27;
  /// Maximum number of entries supported in the table.
  pub(crate) const MAX_ENTRIES: NonZeroUsize = assert_nonzero(1 << Self::MAX_ENTRIES_SHIFT);

  /// Shift used to compute `DEF_ENTRIES`.
  pub(crate) const DEF_ENTRIES_SHIFT: usize = 10;
  /// Default number of entries in a new table.
  pub(crate) const DEF_ENTRIES: NonZeroUsize = assert_nonzero(1 << Self::DEF_ENTRIES_SHIFT);

  #[inline]
  pub(crate) const fn new(capacity: usize) -> Self {
    let length: NonZeroUsize = actual_capacity(capacity);
    let mem_bytes: usize = length.get().strict_mul(Self::ENTRY_SIZE);
    let mem_align: usize = mem_bytes.next_multiple_of(Self::CACHE_LINE);
    let blocks: NonZeroUsize = assert_nonzero(mem_align / Self::CACHE_LINE);

    let bit_width: u32 = bit_width(length.get().strict_sub(1));
    let id_mask_entry: usize = 1_usize.strict_shl(bit_width).strict_sub(1);
    let id_mask_block: usize = blocks.get().strict_sub(1);
    let id_mask_index: usize = Self::SLOT_COUNT.strict_sub(1);
    let id_shift_block: u32 = id_mask_index.trailing_ones();
    let id_shift_index: u32 = id_mask_block.trailing_ones();

    Self {
      blocks,
      length,
      id_mask_entry,
      id_mask_block,
      id_mask_index,
      id_shift_block,
      id_shift_index,
    }
  }
}

#[inline]
const fn actual_capacity(capacity: usize) -> NonZeroUsize {
  let Some(capacity) = capacity.checked_next_power_of_two() else {
    return Params::MAX_ENTRIES;
  };

  if capacity < Params::MIN_ENTRIES.get() {
    Params::MIN_ENTRIES
  } else if capacity > Params::MAX_ENTRIES.get() {
    Params::MAX_ENTRIES
  } else {
    // SAFETY: `capacity` is greater than `MIN_ENTRIES` which is non-zero.
    unsafe { NonZeroUsize::new_unchecked(capacity) }
  }
}

#[inline]
const fn assert_nonzero(value: usize) -> NonZeroUsize {
  NonZeroUsize::new(value).expect("nonzero value")
}

// TODO: Use `usize::bit_width` when stable
#[inline]
const fn bit_width(value: usize) -> u32 {
  usize::BITS.strict_sub(value.leading_zeros())
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_length() {
    // Below minimum
    let params: Params = Params::new(1);
    assert_eq!(params.length, Params::MIN_ENTRIES);

    // Within range - should round to power of two
    let params: Params = Params::new(100);
    assert_eq!(params.length, NonZeroUsize::new(128).unwrap());

    // Exact power of two
    let params: Params = Params::new(256);
    assert_eq!(params.length, NonZeroUsize::new(256).unwrap());

    // Above maximum
    let params: Params = Params::new(1_000_000_000);
    assert_eq!(params.length, Params::MAX_ENTRIES);
  }

  #[test]
  fn test_blocks() {
    for shift in Params::MIN_ENTRIES_SHIFT..=Params::MAX_ENTRIES_SHIFT {
      let params: Params = Params::new(1 << shift);

      // Blocks should be enough to hold `length` entries
      let bytes_expected: usize = params.length.get() * size_of::<Atomic<u64>>();
      let bytes_received: usize = params.blocks.get() * Params::CACHE_LINE;

      assert!(bytes_received >= bytes_expected);
      assert_eq!(bytes_received % Params::CACHE_LINE, 0);
    }
  }

  #[test]
  fn test_length_vs_blocks() {
    for shift in Params::MIN_ENTRIES_SHIFT..=Params::MAX_ENTRIES_SHIFT {
      let params: Params = Params::new(1 << shift);

      let expected: usize = params.blocks.get() * Params::SLOT_COUNT;
      let received: usize = params.length.get();

      assert_eq!(expected, received);
    }
  }

  #[test]
  fn test_masks_and_shifts() {
    for shift in Params::MIN_ENTRIES_SHIFT..=Params::MAX_ENTRIES_SHIFT {
      let params: Params = Params::new(1 << shift);

      // Verify length is power of two
      assert!(params.length.is_power_of_two());

      // Verify blocks is power of two
      assert!(params.blocks.is_power_of_two());

      // Verify bit width relationship
      assert_eq!(
        bit_width(params.length.get() - 1),
        params.id_shift_block + params.id_shift_index,
      );

      // Verify mask composition
      assert_eq!(
        params.id_mask_entry,
        (params.id_mask_block << params.id_shift_block) ^ params.id_mask_index,
      );
    }
  }

  #[test]
  fn test_shifts_match_mask_widths() {
    for shift in Params::MIN_ENTRIES_SHIFT..=Params::MAX_ENTRIES_SHIFT {
      let params: Params = Params::new(1 << shift);

      // id_shift_block should equal the bit width of id_mask_index
      assert_eq!(params.id_shift_block, params.id_mask_index.count_ones());

      // id_shift_index should equal the bit width of id_mask_block
      assert_eq!(params.id_shift_index, params.id_mask_block.count_ones());
    }
  }

  #[test]
  fn test_mask_bits_are_contiguous() {
    for shift in Params::MIN_ENTRIES_SHIFT..=Params::MAX_ENTRIES_SHIFT {
      let params: Params = Params::new(1 << shift);

      // id_mask_entry should have contiguous set bits
      let mask: usize = params.id_mask_entry;
      let bits: u32 = mask.count_ones();
      assert_eq!(mask, (1_usize << bits) - 1);

      // id_mask_block should have contiguous set bits
      let mask: usize = params.id_mask_block;
      let bits: u32 = mask.count_ones();
      assert_eq!(mask, (1_usize << bits) - 1);

      // id_mask_index should have contiguous set bits
      let mask: usize = params.id_mask_index;
      let bits: u32 = mask.count_ones();
      assert_eq!(mask, (1_usize << bits) - 1);
    }
  }

  #[test]
  fn test_entry_mask_covers_all_indices() {
    for shift in Params::MIN_ENTRIES_SHIFT..=Params::MAX_ENTRIES_SHIFT {
      let params: Params = Params::new(1 << shift);

      // Every valid index should fit within the entry mask
      for index in 0..params.length.get() {
        assert!(index <= params.id_mask_entry);
      }
    }
  }
}
