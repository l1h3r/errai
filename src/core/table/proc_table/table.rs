//! Lock-free process table for concurrent process storage and lookup.
//!
//! This module provides [`ProcTable`], a high-performance, lock-free table
//! for storing process data. The table is optimized for concurrent access
//! with cache-line-aware layout to minimize false sharing.
//!
//! # Architecture
//!
//! The table uses a two-level slot allocation scheme:
//!
//! 1. **Abstract indices**: Virtual slot numbers managed via atomic counters
//! 2. **Concrete indices**: Physical memory locations in cache-line-aligned blocks
//!
//! This separation enables lock-free access while maintaining cache efficiency.
//!
//! # Memory Layout
//!
//! Entries are organized into cache-line-sized blocks to reduce contention:
//!
//! ```text
//! ┌──────────┐ ┌──────────┐ ┌───────────┐ ┌───────────┐
//! │ 0 4 8 12 │ │ 1 5 9 13 │ │ 2 6 10 14 │ │ 3 7 11 15 │
//! └──────────┘ └──────────┘ └───────────┘ └───────────┘
//! │Cache-line│ │Cache-line│ │Cache-line │ │Cache-line │
//! ```
//!
//! Sequential allocation spreads entries across blocks, with each block
//! residing in its own cache line.

use crossbeam_epoch as epoch;
use crossbeam_epoch::Atomic;
use crossbeam_epoch::Guard;
use crossbeam_epoch::Owned;
use crossbeam_epoch::Shared;
use crossbeam_utils::CachePadded;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::mem::MaybeUninit;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::AcqRel;
use std::sync::atomic::Ordering::Acquire;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::Ordering::Release;

use crate::core::LocalPid;
use crate::core::ProcAccessError;
use crate::core::ProcInsertError;
use crate::core::table::proc_table::Index;
use crate::core::table::proc_table::Params;
use crate::core::table::proc_table::Permit;
use crate::core::table::proc_table::ReadOnly;
use crate::core::table::proc_table::Volatile;

const RESERVED: usize = usize::MAX;

// -----------------------------------------------------------------------------
// Process Table Iterator
// -----------------------------------------------------------------------------

/// Iterator over the PIDs of currently allocated processes in a [`ProcTable`].
///
/// This iterator provides a snapshot view of allocated PIDs at the time
/// iteration begins. Concurrent insertions or removals do not invalidate
/// the iterator, though yielded PIDs may refer to processes that have
/// since been removed.
pub(crate) struct ProcTableKeys<'table, T> {
  table: &'table ProcTable<T>,
  index: usize,
}

impl<'table, T> ProcTableKeys<'table, T> {
  #[inline]
  const fn new(table: &'table ProcTable<T>) -> Self {
    Self { table, index: 0 }
  }
}

impl<T> Debug for ProcTableKeys<'_, T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("ProcTableKeys(..)")
  }
}

impl<'a, T> Iterator for ProcTableKeys<'a, T> {
  type Item = LocalPid;

  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    while self.index < self.table.capacity() {
      let pid: LocalPid = self.table.readonly.abstract_to_pid(self.index);

      self.index += 1;

      if self.table.exists(pid) {
        return Some(pid);
      }
    }

    None
  }

  #[inline]
  fn size_hint(&self) -> (usize, Option<usize>) {
    let size: usize = self.table.len();
    (size, Some(size))
  }

  #[inline]
  fn count(self) -> usize {
    self.table.len()
  }
}

// -----------------------------------------------------------------------------
// Process Table
// -----------------------------------------------------------------------------

/// Lock-free, cache-line-aware process table storing [`Arc<T>`] entries.
///
/// This table provides concurrent insertion, removal, and lookup of process
/// entries.
///
/// # Capacity
///
/// Table capacity is fixed at creation time and must be between
/// [`MIN_ENTRIES`] and [`MAX_ENTRIES`]. Capacity is automatically rounded
/// to the next power of two.
///
/// [`MIN_ENTRIES`]: Self::MIN_ENTRIES
/// [`MAX_ENTRIES`]: Self::MAX_ENTRIES
#[repr(C)]
pub(crate) struct ProcTable<T> {
  volatile: CachePadded<Volatile>,
  readonly: CachePadded<ReadOnly<T>>,
}

impl<T> ProcTable<T> {
  /// Minimum number of entries supported in the table.
  pub(crate) const MIN_ENTRIES: usize = Params::MIN_ENTRIES.get();

  /// Maximum number of entries supported in the table.
  pub(crate) const MAX_ENTRIES: usize = Params::MAX_ENTRIES.get();

  /// Default number of entries in a new table.
  pub(crate) const DEF_ENTRIES: usize = Params::DEF_ENTRIES.get();

  /// Creates a new, empty table using the default capacity.
  #[inline]
  pub(crate) fn new() -> Self {
    Self::with_capacity(Self::DEF_ENTRIES)
  }

  /// Creates a new, empty table with at least `capacity` slots.
  ///
  /// The actual capacity will be rounded up to the next power of two and
  /// clamped between [`MIN_ENTRIES`] and [`MAX_ENTRIES`].
  ///
  /// [`MIN_ENTRIES`]: Self::MIN_ENTRIES
  /// [`MAX_ENTRIES`]: Self::MAX_ENTRIES
  #[inline]
  pub(crate) fn with_capacity(capacity: usize) -> Self {
    let readonly: ReadOnly<T> = ReadOnly::new(capacity);
    let volatile: Volatile = Volatile::new(&readonly);

    Self {
      volatile: CachePadded::new(volatile),
      readonly: CachePadded::new(readonly),
    }
  }

  /// Returns the maximum number of entries that the table can hold.
  #[inline]
  pub(crate) fn capacity(&self) -> usize {
    self.readonly.concrete_array.len().get()
  }

  /// Returns the total number of entries currently allocated in the table.
  ///
  /// This value may change immediately after reading due to concurrent
  /// operations in other threads.
  #[inline]
  pub(crate) fn len(&self) -> usize {
    self.volatile.len.load(Relaxed) as usize
  }

  /// Returns `true` if the table currently contains no entries.
  #[inline]
  pub(crate) fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Inserts a new entry into the table and returns the assigned PID.
  #[inline]
  pub(crate) fn insert(&self, item: T) -> Result<LocalPid, ProcInsertError> {
    self.write(|uninit, _pid| {
      uninit.write(item);
    })
  }

  /// Inserts a new entry into the table and returns the assigned PID.
  ///
  /// The `init` function receives an uninitialized value and the PID that will
  /// identify this entry. It **must** fully initialize the value before returning.
  ///
  /// # Requirements for `init`
  ///
  /// - **Must** fully initialize the [`MaybeUninit<T>`] before returning
  /// - **Must not** panic (panics will permanently leak a slot)
  /// - **Should** avoid heavy computation or recursive table operations
  ///
  /// # Errors
  ///
  /// Returns [`ProcInsertError`] if the table has reached maximum capacity.
  ///
  /// # Implementation
  ///
  /// 1. Reserve a slot (increment length counter)
  /// 2. Acquire a concrete slot index
  /// 3. Generate PID from the slot index
  /// 4. Allocate uninitialized memory on the heap
  /// 5. Call `init` to initialize the value
  /// 6. Store the initialized value in the table
  /// 7. Return the PID
  ///
  /// # Ordering
  ///
  /// Uses `Release` when storing the entry to ensure the initialized data is
  /// visible to any thread that subsequently reads this slot with `Acquire`.
  #[inline]
  pub(crate) fn write<F>(&self, init: F) -> Result<LocalPid, ProcInsertError>
  where
    F: FnOnce(&mut MaybeUninit<T>, LocalPid),
  {
    let Some(permit) = self.reserve_slot() else {
      return Err(ProcInsertError);
    };

    let abstract_idx: usize = self.acquire_slot(permit);
    let concrete_idx: Index<'_, T> = self.readonly.abstract_to_concrete(abstract_idx);
    let internal_pid: LocalPid = self.readonly.abstract_to_pid(abstract_idx);

    let data: Box<T> = {
      let mut uninit: Box<MaybeUninit<T>> = Box::new_uninit();

      init(&mut uninit, internal_pid);

      // SAFETY: The `init` function is required to fully initialize the value.
      unsafe { uninit.assume_init() }
    };

    let slot: &Atomic<T> = self.readonly.concrete_array.get(concrete_idx);
    let heap: *mut T = Box::into_raw(data);

    debug_assert!(!heap.is_null());
    debug_assert!(heap.is_aligned());

    // SAFETY: Pointer is valid, non-null, and properly aligned.
    let item: Owned<T> = unsafe { Owned::from_raw(heap) };

    slot.store(item, Release);

    Ok(internal_pid)
  }

  /// Removes the entry associated with `pid` and returns `true` if present.
  ///
  /// The slot becomes available for reuse after removal. The actual memory
  /// deallocation is deferred until all current readers have finished.
  ///
  /// # Ordering
  ///
  /// - Slot swap: `AcqRel` (acquire current value, release null to others)
  /// - Free list: `Relaxed` (atomicity only)
  /// - Length: `Release` (make decrement visible)
  #[inline]
  pub(crate) fn remove(&self, pid: LocalPid) -> bool {
    let index: Index<'_, T> = self.readonly.pid_to_concrete(pid);
    let entry: &Atomic<T> = self.readonly.concrete_array.get(index);

    let guard: Guard = epoch::pin();
    let value: Shared<'_, T> = entry.swap(Shared::null(), AcqRel, &guard);

    if value.is_null() {
      return false;
    }

    // SAFETY: The swap gave us ownership. We defer destruction so concurrent
    // readers in `with()` can safely access the data until their guards drop.
    unsafe {
      guard.defer_destroy(value);
    }

    // Let any thread run the destructor
    guard.flush();

    self.release_slot(pid);

    let _ignore: u32 = self.volatile.len.fetch_sub(1, Release);

    true
  }

  /// Returns `true` if an entry exists for the given PID.
  ///
  /// The result may be stale immediately after returning due to concurrent
  /// operations. Another thread may insert or remove this entry between the
  /// check and any subsequent operation.
  #[inline]
  pub(crate) fn exists(&self, pid: LocalPid) -> bool {
    let index: Index<'_, T> = self.readonly.pid_to_concrete(pid);
    let entry: &Atomic<T> = self.readonly.concrete_array.get(index);

    let guard: Guard = epoch::pin();
    let value: Shared<'_, T> = entry.load(Acquire, &guard);

    !value.is_null()
  }

  /// Performs an operation on a process entry with zero-copy access.
  ///
  /// The callback receives a reference to the process data that is guaranteed
  /// to remain valid for the duration of the callback. No cloning occurs.
  ///
  /// # Ordering
  ///
  /// Uses `Acquire` ordering when loading the slot to ensure visibility of the
  /// data written during `insert` with `Release` ordering.
  ///
  /// # Errors
  ///
  /// Returns [`ProcAccessError`] if the process does not exist or the PID is
  /// invalid.
  #[inline]
  pub(crate) fn with<F, R>(&self, pid: LocalPid, f: F) -> Result<R, ProcAccessError>
  where
    F: Fn(&T) -> R,
  {
    let index: Index<'_, T> = self.readonly.pid_to_concrete(pid);
    let entry: &Atomic<T> = self.readonly.concrete_array.get(index);

    let guard: Guard = epoch::pin();
    let value: Shared<'_, T> = entry.load(Acquire, &guard);

    // SAFETY:
    // - The guard ensures the pointer remains valid for its lifetime.
    // - If non-null, the pointer was created by `Owned::from_raw`
    //   during insert and points to a valid, initialized `T`.
    if let Some(data) = unsafe { value.as_ref() } {
      Ok(f(data))
    } else {
      Err(ProcAccessError)
    }
  }

  /// Returns an iterator over all currently allocated PIDs.
  ///
  /// The iterator provides a snapshot view and is not invalidated by
  /// concurrent modifications.
  #[inline]
  pub(crate) fn keys(&self) -> ProcTableKeys<'_, T> {
    ProcTableKeys::new(self)
  }

  /// Translates a PID into its `(number, serial)` components.
  #[inline]
  pub(crate) fn translate_pid(&self, pid: LocalPid) -> (u32, u32) {
    let abstract_idx: usize = self.readonly.pid_to_abstract(pid);

    let number: u32 = (abstract_idx & LocalPid::NUMBER_MASK) as u32;
    let serial: u32 = (abstract_idx >> LocalPid::NUMBER_BITS) as u32;

    (number, serial)
  }

  /// Attempts to reserve a slot in the table.
  ///
  /// Returns a permit that grants the right to claim one slot. The permit does
  /// not identify which slot will be claimed - that is determined later during
  /// acquisition.
  ///
  /// # Implementation
  ///
  /// 1. Optimistically increment the length counter
  /// 2. If the increment stays within capacity, return a permit
  /// 3. Otherwise, use a CAS loop to restore the counter to capacity
  ///
  /// # Ordering
  ///
  /// Uses `Relaxed` ordering throughout:
  ///
  /// - Increment only needs atomicity, not synchronization
  /// - Capacity check is against immutable readonly data
  /// - Failed reservations only need atomicity to undo the increment
  #[inline]
  fn reserve_slot(&self) -> Option<Permit<'_, T>> {
    let prev: u32 = self.volatile.len.fetch_add(1, Relaxed);

    if prev < self.capacity() as u32 {
      return Some(Permit::new(self));
    }

    let mut current: u32 = prev + 1;

    loop {
      match self
        .volatile
        .len
        .compare_exchange_weak(current, current - 1, Relaxed, Relaxed)
      {
        Ok(_) => break,
        Err(next) => current = next,
      }
    }

    None
  }

  /// Acquires a concrete slot for a reserved permit.
  ///
  /// Scans the abstract array to find an unreserved slot, claims it, and
  /// returns the index where the entry should be stored.
  ///
  /// # Implementation
  ///
  /// 1. Get next abstract index from allocation counter
  /// 2. Map abstract index to concrete index (cache-line aware layout)
  /// 3. Attempt to claim the slot by swapping in `RESERVED` marker
  /// 4. If already reserved by another thread, retry with next index
  /// 5. Return the concrete index that was successfully claimed
  ///
  /// # Ordering
  ///
  /// - Allocation counter: `Relaxed` (atomicity only, permits prevent races)
  /// - Slot claim: `AcqRel` (acquire to see if reserved, release to publish claim)
  #[inline]
  fn acquire_slot(&self, _permit: Permit<'_, T>) -> usize {
    loop {
      let abstract_idx: u32 = self.volatile.aid.fetch_add(1, Relaxed);
      let concrete_idx: Index<'_, T> = self.readonly.abstract_to_concrete(abstract_idx as usize);

      let atomic: &AtomicUsize = self.readonly.abstract_array.get(concrete_idx);
      let result: usize = atomic.swap(RESERVED, AcqRel);

      if result != RESERVED {
        return result;
      }
    }
  }

  /// Releases a slot previously reserved by PID.
  #[inline]
  fn release_slot(&self, pid: LocalPid) {
    let data: usize = self.generate_next_slot(pid);

    loop {
      let abstract_idx: u32 = self.volatile.fid.fetch_add(1, Relaxed);
      let concrete_idx: Index<'_, T> = self.readonly.abstract_to_concrete(abstract_idx as usize);

      let atomic: &AtomicUsize = self.readonly.abstract_array.get(concrete_idx);
      let result: Result<usize, usize> = atomic.compare_exchange(RESERVED, data, Relaxed, Relaxed);

      if result.is_ok() {
        break;
      }
    }
  }

  #[inline]
  fn generate_next_slot(&self, pid: LocalPid) -> usize {
    let mut data: usize = self.readonly.pid_to_abstract(pid);

    data += self.capacity();
    data &= LocalPid::PID_MASK;

    // Ensure we never store the reserved marker value.
    if data == RESERVED {
      data += self.capacity();
      data &= LocalPid::PID_MASK;
    }

    data
  }
}

impl<T> Drop for ProcTable<T> {
  fn drop(&mut self) {
    let guard: Guard = epoch::pin();

    for entry in self.readonly.concrete_array.as_slice() {
      let value: Shared<'_, T> = entry.load(Acquire, &guard);

      // SAFETY: We have exclusive access during drop.
      if let Some(owned) = unsafe { value.try_into_owned() } {
        drop(owned.into_box());
      }
    }
  }
}

impl<T> Debug for ProcTable<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    struct Mask(usize);

    impl Debug for Mask {
      fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
          f,
          "{bits:0>size$b}",
          bits = &self.0,
          size = usize::BITS as usize,
        )
      }
    }

    f.debug_struct("ProcTable")
      .field("v:len", &self.volatile.len)
      .field("v:aid", &self.volatile.aid)
      .field("v:fid", &self.volatile.fid)
      .field("r:concrete_array", &self.readonly.concrete_array.as_ptr())
      .field("r:abstract_array", &self.readonly.abstract_array.as_ptr())
      .field("r:total_capacity", &self.readonly.concrete_array.len())
      .field("r:id_mask_entry", &Mask(self.readonly.id_mask_entry))
      .field("r:id_mask_block", &Mask(self.readonly.id_mask_block))
      .field("r:id_mask_index", &Mask(self.readonly.id_mask_index))
      .field("r:id_shift_block", &self.readonly.id_shift_block)
      .field("r:id_shift_index", &self.readonly.id_shift_index)
      .finish()
  }
}

unsafe impl<T: Send> Send for ProcTable<T> {}
unsafe impl<T: Send> Sync for ProcTable<T> {}

impl<T> RefUnwindSafe for ProcTable<T> {}
impl<T> UnwindSafe for ProcTable<T> {}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::collections::HashSet;

  use super::*;

  type ProcTable = super::ProcTable<usize>;

  #[test]
  fn test_new() {
    let table: ProcTable = ProcTable::new();

    assert_eq!(table.capacity(), ProcTable::DEF_ENTRIES);
    assert_eq!(table.len(), 0);
    assert!(table.is_empty());
  }

  #[test]
  fn test_with_capacity_rounds_up_pow2() {
    let table: ProcTable = ProcTable::with_capacity(100);
    assert_eq!(table.capacity(), 128);

    let table: ProcTable = ProcTable::with_capacity(1000);
    assert_eq!(table.capacity(), 1024);

    let table: ProcTable = ProcTable::with_capacity(1025);
    assert_eq!(table.capacity(), 2048);
  }

  #[test]
  fn test_with_capacity_clamp_min() {
    let table: ProcTable = ProcTable::with_capacity(0);
    assert_eq!(table.capacity(), ProcTable::MIN_ENTRIES);

    let table: ProcTable = ProcTable::with_capacity(1);
    assert_eq!(table.capacity(), ProcTable::MIN_ENTRIES);

    let table: ProcTable = ProcTable::with_capacity(8);
    assert_eq!(table.capacity(), ProcTable::MIN_ENTRIES);
  }

  #[test]
  fn test_with_capacity_clamp_max() {
    let table: ProcTable = ProcTable::with_capacity(usize::MAX);
    assert_eq!(table.capacity(), ProcTable::MAX_ENTRIES);

    let table: ProcTable = ProcTable::with_capacity(1 << 30);
    assert_eq!(table.capacity(), ProcTable::MAX_ENTRIES);
  }

  #[test]
  fn test_with_capacity_exact_power_of_two() {
    for shift in Params::MIN_ENTRIES_SHIFT..=Params::MAX_ENTRIES_SHIFT {
      let table: ProcTable = ProcTable::with_capacity(1 << shift);
      assert_eq!(table.capacity(), 1 << shift);
    }
  }

  #[test]
  fn test_insert_single() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let local_pid: LocalPid = table.insert(123).unwrap();

    assert_eq!(table.len(), 1);
    assert!(!table.is_empty());
    assert!(table.exists(local_pid));
    assert_eq!(table.with(local_pid, |data| *data), Ok(123));
  }

  #[test]
  fn test_insert_callback_receives_correct_pid() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);

    let out: LocalPid = table
      .write(|uninit, pid| {
        uninit.write(pid.into_bits());
      })
      .unwrap();

    assert_eq!(table.with(out, |data| *data), Ok(out.into_bits()));
  }

  #[test]
  fn test_insert_multiple() {
    let table: ProcTable = ProcTable::with_capacity(64);
    let mut pids: Vec<LocalPid> = Vec::new();

    for index in 0..32 {
      pids.push(table.insert(index * 100).unwrap());
    }

    assert_eq!(table.len(), 32);

    // Verify all values
    for (index, pid) in pids.iter().enumerate() {
      assert_eq!(table.with(*pid, |data| *data), Ok(index * 100));
    }
  }

  #[test]
  fn test_insert_unique_pids() {
    let table: ProcTable = ProcTable::with_capacity(64);
    let mut pids: HashSet<usize> = HashSet::new();

    for _ in 0..32 {
      let pid: LocalPid = table.insert(0).unwrap();
      let new: bool = pids.insert(pid.into_bits());

      assert!(new, "duplicate PID generated");
    }
  }

  #[test]
  fn test_insert_maximum() {
    let table: ProcTable = ProcTable::with_capacity(32);

    for index in 0..table.capacity() {
      assert!(table.insert(index).is_ok());
    }

    assert_eq!(table.len(), 32);
    assert_eq!(table.insert(9999), Err(ProcInsertError));
  }

  #[test]
  fn test_remove_existing() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let local_pid: LocalPid = table.insert(123).unwrap();

    assert_eq!(table.len(), 1);
    assert!(table.exists(local_pid));

    assert!(
      table.remove(local_pid),
      "remove should return true for existing entry",
    );

    assert_eq!(table.len(), 0);
    assert!(table.is_empty());
    assert!(!table.exists(local_pid));
  }

  #[test]
  fn test_remove_nonexistent() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let local_pid: LocalPid = table.insert(123).unwrap();

    table.remove(local_pid);

    assert!(
      !table.remove(local_pid),
      "remove should return false for nonexistent entry",
    );
  }

  #[test]
  fn test_remove_isolation() {
    let table: ProcTable = ProcTable::with_capacity(32);
    let mut pids: Vec<LocalPid> = Vec::new();

    for index in 0..16 {
      pids.push(table.insert(index).unwrap());
    }

    for index in (0..16).step_by(2) {
      table.remove(pids[index]);
    }

    assert_eq!(table.len(), 8);

    for index in (1..16).step_by(2) {
      assert_eq!(table.with(pids[index], |data| *data), Ok(index));
    }

    for index in (0..16).step_by(2) {
      assert!(!table.exists(pids[index]));
    }
  }

  #[test]
  fn test_remove_recycling() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let mut pids: Vec<LocalPid> = Vec::new();

    for index in 0..table.capacity() {
      pids.push(table.insert(index).unwrap());
    }

    assert_eq!(table.insert(99), Err(ProcInsertError));

    table.remove(pids[0]);

    let new: LocalPid = table.insert(100).unwrap();

    assert!(table.exists(new));
    assert_eq!(table.with(new, |data| *data), Ok(100));
  }

  #[test]
  fn test_exists_existing() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let local_pid: LocalPid = table.insert(123).unwrap();

    assert!(table.exists(local_pid));
  }

  #[test]
  fn test_exists_nonexistent() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let local_pid: LocalPid = table.insert(123).unwrap();

    assert!(table.remove(local_pid));
    assert!(!table.exists(local_pid));
  }

  #[test]
  fn test_exists_multiple() {
    let table: ProcTable = ProcTable::with_capacity(32);

    let pid1: LocalPid = table.insert(1).unwrap();
    let pid2: LocalPid = table.insert(2).unwrap();
    let pid3: LocalPid = table.insert(3).unwrap();

    assert!(table.exists(pid1));
    assert!(table.exists(pid2));
    assert!(table.exists(pid3));

    table.remove(pid2);

    assert!(table.exists(pid1));
    assert!(!table.exists(pid2));
    assert!(table.exists(pid3));
  }

  #[test]
  fn test_with_value() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let local_pid: LocalPid = table.insert(12345).unwrap();

    assert_eq!(table.with(local_pid, |data| *data), Ok(12345));
  }

  #[test]
  fn test_with_return_value() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let local_pid: LocalPid = table.insert(123).unwrap();

    assert_eq!(table.with(local_pid, |data| data + 1), Ok(124));
  }

  #[test]
  fn test_with_nonexistent() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let local_pid: LocalPid = table.insert(123).unwrap();

    assert!(table.remove(local_pid));
    assert_eq!(table.with(local_pid, |data| *data), Err(ProcAccessError));
  }

  #[test]
  fn test_with_multiple() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let local_pid: LocalPid = table.insert(123).unwrap();

    for _ in 0..100 {
      assert_eq!(table.with(local_pid, |data| *data), Ok(123));
    }
  }

  #[test]
  fn test_len_tracks_insertions() {
    let table: ProcTable = ProcTable::with_capacity(32);

    for index in 0..16 {
      table.insert(0).unwrap();
      assert_eq!(table.len(), index + 1);
    }
  }

  #[test]
  fn test_len_tracks_removals() {
    let table: ProcTable = ProcTable::with_capacity(32);
    let mut pids: Vec<LocalPid> = Vec::new();

    for _ in 0..16 {
      pids.push(table.insert(0).unwrap());
    }

    for (index, pid) in pids.iter().enumerate() {
      table.remove(*pid);
      assert_eq!(table.len(), 16 - index - 1);
    }
  }

  #[test]
  fn test_is_empty() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);

    assert!(table.is_empty());

    let local_pid: LocalPid = table.insert(0).unwrap();

    assert!(!table.is_empty());
    assert!(table.remove(local_pid));
    assert!(table.is_empty());
  }

  #[test]
  fn test_interleaved_insert_remove() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let mut pids: Vec<LocalPid> = Vec::new();

    for index in 0..8 {
      pids.push(table.insert(index).unwrap());
    }
    assert_eq!(table.len(), 8);

    for _ in 0..4 {
      table.remove(pids.pop().unwrap());
    }
    assert_eq!(table.len(), 4);

    for index in 100..108 {
      pids.push(table.insert(index).unwrap());
    }
    assert_eq!(table.len(), 12);

    for pid in pids {
      assert!(table.exists(pid));
    }
  }

  #[test]
  fn test_multiple_refills() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);

    for round in 0..3 {
      let mut pids: Vec<LocalPid> = Vec::new();

      for index in 0..16 {
        pids.push(table.insert(round * 100 + index).unwrap());
      }
      assert_eq!(table.len(), 16);

      for (index, pid) in pids.iter().enumerate() {
        assert_eq!(table.with(*pid, |data| *data), Ok(round * 100 + index));
      }

      for pid in pids {
        table.remove(pid);
      }
      assert_eq!(table.len(), 0);
    }
  }

  #[test]
  fn test_pid_uniqueness_multiple_generations() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MIN_ENTRIES);
    let mut all_pids: HashSet<usize> = HashSet::new();

    for _ in 0..10 {
      let mut round_pids: Vec<LocalPid> = Vec::new();

      for _ in 0..16 {
        let pid: LocalPid = table.insert(0).unwrap();

        round_pids.push(pid);

        assert!(
          all_pids.insert(pid.into_bits()),
          "PID reused across generations",
        );
      }

      for pid in round_pids {
        table.remove(pid);
      }
    }
  }

  #[test]
  fn test_min_capacity_operations() {
    let table: ProcTable = ProcTable::with_capacity(1);
    assert_eq!(table.capacity(), ProcTable::MIN_ENTRIES);

    let pid: LocalPid = table.insert(99).unwrap();

    assert!(table.exists(pid));
    assert_eq!(table.with(pid, |data| *data), Ok(99));

    assert!(table.remove(pid));
    assert!(!table.exists(pid));
  }

  #[test]
  fn test_max_capacity_operations() {
    let table: ProcTable = ProcTable::with_capacity(ProcTable::MAX_ENTRIES);
    assert_eq!(table.capacity(), ProcTable::MAX_ENTRIES);
    assert_eq!(table.len(), 1); // See `Volatile::new`
  }
}
