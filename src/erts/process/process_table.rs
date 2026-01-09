use crossbeam_utils::CachePadded;
use std::alloc::Layout;
use std::alloc::alloc;
use std::alloc::dealloc;
use std::alloc::handle_alloc_error;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::hint;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::num::NonZeroUsize;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;
use std::ptr::NonNull;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::AcqRel;
use std::sync::atomic::Ordering::Acquire;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::Ordering::Release;
use triomphe::Arc;
use triomphe::UniqueArc;

use crate::lang::RawPid;
use crate::tyre::ptr::AtomicTaggedPtr;
use crate::tyre::ptr::TaggedPtr;

// -----------------------------------------------------------------------------
// Table Full Error
// -----------------------------------------------------------------------------

/// Error returned by [`ProcessTable::insert`].
#[derive(Debug)]
#[non_exhaustive]
pub struct ProcessTableFull;

impl Display for ProcessTableFull {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("too many processes")
  }
}

impl Error for ProcessTableFull {}

// -----------------------------------------------------------------------------
// Table Keys Iterator
// -----------------------------------------------------------------------------

pub struct Keys<'a, T> {
  table: &'a ProcessTable<T>,
  index: usize,
}

impl<'a, T> Keys<'a, T> {
  #[inline]
  const fn new(table: &'a ProcessTable<T>) -> Self {
    Self { table, index: 0 }
  }
}

impl<T> Debug for Keys<'_, T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("Keys(..)")
  }
}

impl<'a, T> Iterator for Keys<'a, T> {
  type Item = RawPid;

  #[inline]
  fn next(&mut self) -> Option<Self::Item> {
    while self.index < self.table.capacity() {
      let index: usize = self.index;

      self.index += 1;

      if self.table.has(index) {
        return Some(self.table.readonly.base_index_to_pid(index as u64));
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

/// Cache-Line-Aware Lock-Free Process Table
///
/// A fixed-capacity, lock-free table for storing pointers to processes,
/// optimized for concurrent access and cache-friendly memory layout.
///
/// ## Guarantees
///
/// - Lock-free insertion, removal, and lookup
/// - Starvation-resistant slot allocation and recycling
/// - Readers always observe fully initialized entries (no torn reads)
/// - Cache-friendly layout minimizes false sharing
/// - Strict memory ordering guarantees (Acquire/Release/AcqRel)
///
/// ## Memory Model
///
/// - **Insertion**: Initialize `Arc<T>`, then store in the table with a `Release` store.
/// - **Lookup**: Load entry using `Acquire` ordering, ensuring visibility of all writes.
/// - **Removal**: Atomically mark entry as `DEAD` with `AcqRel` ordering. Entries in `DEAD` state
///   are not returned to readers.
/// - **Slot Recycling**: Monotonic atomic counters (`aid`, `fid`) manage slot reservation and reuse,
///   ensuring visibility of generation updates using `Acquire` and `Release` ordering.
///
/// ## Safety and Invariants
///
/// - The table must not be dropped while threads may access it.
/// - Tagged pointers must never be dereferenced directly. They are used to encode the state of slots.
///
/// ### Tagged Pointer Invariants
///
/// - **Tags**: Entries store raw pointers with the low-order bits used as tags:
///   - `0b00`: FREE (slot is available for insertion)
///   - `0b01`: USED (slot is in use)
///   - `0b10`: DEAD (slot is logically removed)
///
/// - Operations on tagged pointers must adhere to:
///   - `TaggedPtr::with(ptr, tag)`: Preserves the original pointer in non-tag bits.
///   - `TaggedPtr::tag(ptr)`: Returns only tag bits.
///   - `TaggedPtr::del(ptr)`: Retrieves the original `Arc<T>` data pointer.
///   - **Violated Invariants**: Dereferencing tagged pointers without untagging or improperly
///     modifying the tag bits leads to undefined behavior.
///
/// ## Arc Lifetime Safety
///
/// - Every entry holds an `Arc<T>`, ensuring safe memory management and preventing use-after-free.
///   The **Arc's reference counting** ensures that the memory for the stored process is properly
///   deallocated when no longer needed, while preventing data races.
///
/// ## Concurrency and Synchronization
///
/// - The table supports **lock-free** concurrent access by multiple threads.
/// - **Insertion**, **lookup**, and **removal** operations are atomic, ensuring that threads can operate
///   without locks while maintaining correctness. This lock-free design prevents deadlocks and reduces
///   contention in high-concurrency scenarios.
/// - The **volatile section** is designed to minimize false sharing and ensures efficient concurrent access
///   by frequently updated atomics, while the **read-only section** contains static data that's not modified
///   after initialization.
///
/// ## Layout (Maintainer Reference)
///
/// The table is divided into two cache-line-aligned sections:
///
/// - **Volatile Section**: Contains atomic counters and frequently updated atomics for slot
///   reservation and tracking.
/// - **Read-only Section**: Contains static metadata and storage pointers, which are not modified
///   after initialization.
///
/// Entries are organized into **cache-line-sized blocks**:
/// - Each block contains multiple **slots**, each holding an atomic pointer.
/// - Blocks are allocated contiguously in memory.
/// - Slot allocation proceeds sequentially through blocks, with a generation counter used to
///   track slot reuse across blocks.
///
/// ### Visual Representation
///
/// Example table with 4 blocks and 4 slots per block:
///
/// ┌──────────┐ ┌──────────┐ ┌───────────┐ ┌───────────┐
/// │ 0 4 8 12 │ │ 1 5 9 13 │ │ 2 6 10 14 │ │ 3 7 11 15 │
/// └──────────┘ └──────────┘ └───────────┘ └───────────┘
///
/// - Boxes represent blocks, and numbers represent logical slot indices.
/// - Sequentially inserted entries fill slots across blocks, wrapping around when full.
#[repr(C)]
pub struct ProcessTable<T> {
  volatile: CachePadded<Volatile>,
  readonly: CachePadded<ReadOnly<T>>,
}

impl<T> ProcessTable<T> {
  /// Minimum number of entries supported in the table.
  pub const MIN_ENTRIES: usize = 1 << 4;

  // Maximum number of entries supported in the table.
  pub const MAX_ENTRIES: usize = 1 << 27;

  /// Default number of entries supported in the table.
  #[cfg(not(test))]
  pub const DEF_ENTRIES: usize = 1 << 20;
  #[cfg(test)]
  pub const DEF_ENTRIES: usize = 1 << 10;

  /// Creates a new, empty `ProcessTable` with the default capacity.
  #[inline]
  pub fn new() -> Self {
    Self::with_capacity(Self::DEF_ENTRIES)
  }

  /// Creates a new, empty `ProcessTable` with at least the specified capacity.
  ///
  /// The table will not reallocate, and will hold at least `capacity` entries.
  pub fn with_capacity(capacity: usize) -> Self {
    Self::alloc_table(capacity)
  }

  /// Returns the maximum number of entries that the table can hold.
  #[inline]
  pub fn capacity(&self) -> usize {
    self.readonly.cap.get()
  }

  /// Returns the number of entries currently in the table.
  #[inline]
  pub fn len(&self) -> usize {
    self.volatile.len.load(Relaxed) as usize
  }

  /// Returns `true` if the table contains no entries.
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Adds a new entry to the table, returning an `Arc<T>` to the added entry.
  ///
  /// The provided `init` function is called to initialize the new entry
  /// **after** space for it has been reserved.
  ///
  /// # Requirements for the `init` function:
  ///
  /// - Must never fail; failure results in a permanently lost slot.
  /// - Avoid heavy computation or recursive table calls to prevent deadlocks.
  /// - Must fully initialize the `MaybeUninit<T>` before returning.
  ///
  /// # Errors
  ///
  /// Returns [`ProcessTableFull`] if the table has reached its maximum capacity.
  #[inline]
  pub fn insert<F>(&self, init: F) -> Result<Arc<T>, ProcessTableFull>
  where
    F: FnOnce(&mut MaybeUninit<T>, RawPid),
  {
    self.do_insert(init)
  }

  /// Removes and returns the entry associated with the given `pid`,
  /// or `None` if not found.
  ///
  /// This method may remove a previously inserted entry at the same slot if the
  /// `pid` matches an older entry. This behavior is safe but may be surprising
  /// to some users. The method ensures that the entry is either fully removed
  /// or returned if still valid.
  pub fn remove(&self, pid: RawPid) -> Option<Arc<T>> {
    self.do_remove(pid, self.readonly.pid_to_real_index(pid))
  }

  /// Checks if an existing entry is associated with the given `pid` and not
  /// marked for removal.
  ///
  /// Note: This function checks the entry's existence, but it does **not**
  /// guarantee that the entry will remain valid after the call returns. Users
  /// should not assume the entry remains available.
  #[inline]
  pub fn exists(&self, pid: RawPid) -> bool {
    self.do_exists(self.readonly.pid_to_real_index(pid))
  }

  /// Retrieves the entry associated with the given `pid`, if it exists and is
  /// not marked for removal.
  #[inline]
  pub fn get(&self, pid: RawPid) -> Option<Arc<T>> {
    self.do_get(self.readonly.pid_to_real_index(pid))
  }

  /// Checks if an entry exists at the given `index` and is not marked for removal.
  #[inline]
  pub fn has(&self, index: usize) -> bool {
    if index < self.capacity() {
      self.do_exists(self.readonly.base_index_to_real_index(index as u64))
    } else {
      false
    }
  }

  /// Returns an iterator over all the keys (PIDs) currently in the table.
  #[inline]
  pub fn keys(&self) -> Keys<'_, T> {
    Keys::new(self)
  }

  /// Translates the given `pid` into a 2-tuple containing the `number` and
  /// `serial` components.
  #[inline]
  pub fn translate_pid(&self, pid: RawPid) -> (u32, u32) {
    fn bits(value: u64, start: u32, count: u32) -> u64 {
      (value >> start) & !(!0_u64 << count)
    }

    let source: u64 = self.readonly.pid_to_base_index(pid);
    let number: u32 = bits(source, 0, RawPid::NUMBER_BITS) as u32;
    let serial: u32 = bits(source, RawPid::NUMBER_BITS, RawPid::SERIAL_BITS) as u32;

    (number, serial)
  }
}

// -----------------------------------------------------------------------------
// Process Table - Internals
// -----------------------------------------------------------------------------

unsafe impl<T: Send> Send for ProcessTable<T> {}
unsafe impl<T: Send> Sync for ProcessTable<T> {}

impl<T> RefUnwindSafe for ProcessTable<T> {}
impl<T> UnwindSafe for ProcessTable<T> {}

impl<T> Drop for ProcessTable<T> {
  fn drop(&mut self) {
    let memory: usize = strict_align(self.capacity().strict_mul(ENTRY_SIZE));
    let layout: Layout = Layout::from_size_align(memory, CACHE_LINE).expect("valid layout");

    // SAFETY: The `ptr_concrete` and `ptr_abstract` pointers are guaranteed to
    //         be valid, as they are both allocated using the internal `alloc`
    //         function. The `dealloc_*` functions are therefore safe as we use
    //         the same computed layout to deallocate.
    unsafe {
      Self::dealloc_concrete_array(layout, self.readonly.ptr_concrete);
      Self::dealloc_abstract_array(layout, self.readonly.ptr_abstract);
    }
  }
}

impl<T> Debug for ProcessTable<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    struct Mask(u32);

    impl Debug for Mask {
      fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:0>32b}", &self.0)
      }
    }

    f.debug_struct("ProcessTable")
      .field("volatile.len", &self.volatile.len)
      .field("volatile.aid", &self.volatile.aid)
      .field("volatile.fid", &self.volatile.fid)
      .field("readonly.ptr_concrete", &self.readonly.ptr_concrete)
      .field("readonly.ptr_abstract", &self.readonly.ptr_abstract)
      .field("readonly.cap", &self.readonly.cap)
      .field("readonly.id_mask_entry", &Mask(self.readonly.id_mask_entry))
      .field("readonly.id_mask_block", &Mask(self.readonly.id_mask_block))
      .field("readonly.id_mask_index", &Mask(self.readonly.id_mask_index))
      .field("readonly.id_shift_block", &self.readonly.id_shift_block)
      .field("readonly.id_shift_index", &self.readonly.id_shift_index)
      .finish()
  }
}

// -----------------------------------------------------------------------------
// Process Table - Slot Allocation
// -----------------------------------------------------------------------------

impl<T> ProcessTable<T> {
  fn do_insert<F>(&self, init: F) -> Result<Arc<T>, ProcessTableFull>
  where
    F: FnOnce(&mut MaybeUninit<T>, RawPid),
  {
    // -------------------------------------------------------------------------
    // 1. Reserve Slot
    // -------------------------------------------------------------------------

    'reserve: {
      // Relaxed ensures that the atomic increment of len happens atomically,
      // but there are no guarantees about synchronization or visibility with
      // respect to other operations. This is fine as we only care about
      // atomicity here.
      let count: u32 = self.volatile.len.fetch_add(1, Relaxed);

      if count < self.capacity() as u32 {
        break 'reserve;
      }

      let mut count: u32 = count + 1;

      // Relaxed orderings are used for both `compare_exchange` operations. This
      // is valid as we're only concerned with atomicity, not visibility or
      // synchronization between threads. We don't need to enforce any memory
      // ordering across threads here.
      'undo: while let Err(next) =
        self
          .volatile
          .len
          .compare_exchange(count, count - 1, Relaxed, Relaxed)
      {
        if next <= self.capacity() as u32 {
          break 'undo;
        }

        count = next;
      }

      return Err(ProcessTableFull);
    }

    // -------------------------------------------------------------------------
    // 2. Allocate Entry
    // -------------------------------------------------------------------------

    let mut uninit: UniqueArc<MaybeUninit<T>> = UniqueArc::new_uninit();

    // -------------------------------------------------------------------------
    // 3. Load Slot Index
    // -------------------------------------------------------------------------

    let base_index: u64 = 'acquire: loop {
      // Relaxed ensures that the increment operation is atomic, but there are
      // no memory ordering guarantees. This is acceptable since no
      // synchronization or visibility between threads is needed.
      let base_index: u64 = self.volatile.aid.fetch_add(1, Relaxed);
      let real_index: Index<'_, T> = self.readonly.base_index_to_real_index(base_index);

      // AcqRel ensures that any memory operations before the swap are visible
      // to the thread performing the swap (acquire). It also ensures that any
      // changes after the swap will be visible to other threads (release).
      let atomic: &AtomicU64 = self.readonly.get_abstract(real_index);
      let result: u64 = atomic.swap(RESERVED, AcqRel);

      if result == RESERVED {
        continue 'acquire;
      }

      break 'acquire result;
    };

    let real_index: Index<'_, T> = self.readonly.base_index_to_real_index(base_index);

    debug_assert!(
      real_index.get() < self.capacity() as u64,
      "ProcessTable::do_insert requires that the index is in bounds",
    );

    // -------------------------------------------------------------------------
    // 4. Initialize Entry
    // -------------------------------------------------------------------------

    init(&mut uninit, self.readonly.base_index_to_pid(base_index));

    // -------------------------------------------------------------------------
    // 5. Write Entry
    // -------------------------------------------------------------------------

    // SAFETY: `uninit` was initialized by the provided `init` function.
    let data_unique: UniqueArc<T> = unsafe { UniqueArc::assume_init(uninit) };
    let data_shared: Arc<T> = UniqueArc::shareable(data_unique);

    let slot: &AtomicTaggedPtr<T> = self.readonly.get_concrete(real_index);
    let heap: *mut T = Arc::into_raw(Arc::clone(&data_shared)).cast_mut();
    // SAFETY: The alignment of `TaggedPtr<T>` was asserted during setup.
    let item: TaggedPtr<T> = unsafe { TaggedPtr::with_unchecked(heap, PTR_TAG_FREE) };

    debug_assert!(
      !item.is_null(),
      "ProcessTable::do_insert requires that the pointer is not NULL",
    );

    debug_assert!(
      item.is_aligned(),
      "ProcessTable::do_get requires that the pointer is aligned",
    );

    // Release ensures that all prior memory operations (like initializing the
    // entry) are visible to other threads once this operation is complete. This
    // is crucial to ensure that any writes to the table entry are visible
    // before marking it as free.
    slot.store(item, Release);

    // -------------------------------------------------------------------------
    // 6. Return
    // -------------------------------------------------------------------------

    Ok(data_shared)
  }

  fn do_remove(&self, pid: RawPid, index: Index<'_, T>) -> Option<Arc<T>> {
    debug_assert!(
      index.get() < self.capacity() as u64,
      "ProcessTable::do_remove requires that the index is in bounds",
    );

    // -------------------------------------------------------------------------
    // 1. Load Slot
    // -------------------------------------------------------------------------

    let slot: &AtomicTaggedPtr<T> = self.readonly.get_concrete(index);

    // -------------------------------------------------------------------------
    // 2. Spin
    // -------------------------------------------------------------------------

    'spin: loop {
      // -----------------------------------------------------------------------
      // 1. Load Entry
      // -----------------------------------------------------------------------

      // Acquire ensures that all memory operations before the load are visible
      // to the current thread. This guarantees that we are reading the most
      // up-to-date value.
      let item: TaggedPtr<T> = slot.load(Acquire);

      // -----------------------------------------------------------------------
      // 2. Validation
      // -----------------------------------------------------------------------

      if item.is_null() || item.tag() == PTR_TAG_DEAD {
        return None;
      }

      if item.tag() != PTR_TAG_FREE {
        hint::spin_loop();
        continue 'spin;
      }

      // -----------------------------------------------------------------------
      // 3. Lock Entry
      // -----------------------------------------------------------------------

      // AcqRel ensures that prior operations (like marking the entry as free)
      // are visible before this exchange happens. Acquire ensures that
      // subsequent reads after the exchange will see the updated entry.
      let lock: Result<TaggedPtr<T>, TaggedPtr<T>> =
        slot.compare_exchange(item, item.set(PTR_TAG_USED), AcqRel, Acquire);

      if lock.is_err() {
        hint::spin_loop();
        continue 'spin;
      }

      debug_assert!(
        slot.load(Acquire).tag() == PTR_TAG_USED,
        "ProcessTable::do_remove requires that the entry is locked",
      );

      // -----------------------------------------------------------------------
      // 4. Release Slot
      // -----------------------------------------------------------------------

      let data: u64 = self.refresh_data(pid);

      debug_assert!(
        data != RESERVED,
        "ProcessTable::do_remove requires that the slot data is valid",
      );

      debug_assert!(
        index == self.readonly.base_index_to_real_index(data),
        "ProcessTable::do_remove requires that the slot mapping is reversible",
      );

      'release: loop {
        // Relaxed ensures that the increment operation is atomic, but there are
        // no memory ordering guarantees. This is acceptable since no
        // synchronization or visibility between threads is needed.
        let base_index: u64 = self.volatile.fid.fetch_add(1, Relaxed);
        let real_index: Index<'_, T> = self.readonly.base_index_to_real_index(base_index);

        // Relaxed ensures atomicity of the comparison and exchange without
        // imposing synchronization between threads. Since we're only updating
        // the state of the slot, no additional ordering is necessary.
        let atomic: &AtomicU64 = self.readonly.get_abstract(real_index);
        let result: Result<u64, u64> = atomic.compare_exchange(RESERVED, data, Relaxed, Relaxed);

        if result.is_ok() {
          break 'release;
        }
      }

      debug_assert!(
        self.len() > 0,
        "ProcessTable::do_remove requires that length is > 0",
      );

      // Release ensures that all memory operations before this operation are
      // visible to other threads after the decrement. This is important to
      // ensure visibility of the decrement operation after the length change.
      let _update_len: u32 = self.volatile.len.fetch_sub(1, Release);

      // -----------------------------------------------------------------------
      // 5. Read Entry
      // -----------------------------------------------------------------------

      debug_assert!(
        !item.is_null(),
        "ProcessTable::do_remove requires that the pointer is not NULL",
      );

      debug_assert!(
        item.is_aligned(),
        "ProcessTable::do_remove requires that the pointer is aligned",
      );

      // SAFETY: `ptr` is derived from a valid `Arc<T>` and assumed to be properly initialized.
      let data_source: Arc<T> = unsafe { Arc::from_raw(item.as_ptr()) };

      // -----------------------------------------------------------------------
      // 6. Delete Entry
      // -----------------------------------------------------------------------

      // Release ensures that all prior memory operations (like marking the
      // entry for removal) are visible to other threads once the store
      // completes. This ensures that other threads see the removal after the
      // store finishes, but no memory ordering is imposed after the store.
      slot.store(TaggedPtr::null(), Release);

      // -----------------------------------------------------------------------
      // 7. Return
      // -----------------------------------------------------------------------

      break 'spin Some(data_source);
    }
  }

  fn do_exists(&self, index: Index<'_, T>) -> bool {
    debug_assert!(
      index.get() < self.capacity() as u64,
      "ProcessTable::do_exists requires that the index is in bounds",
    );

    // -------------------------------------------------------------------------
    // 1. Load Slot/Entry
    // -------------------------------------------------------------------------

    // Acquire ensures that any operations before the load (such as marking the
    // entry for removal) are visible to the current thread. This guarantees
    // that we load the most up-to-date state.
    let slot: &AtomicTaggedPtr<T> = self.readonly.get_concrete(index);
    let item: TaggedPtr<T> = slot.load(Acquire);

    // -------------------------------------------------------------------------
    // 2. Validate
    // -------------------------------------------------------------------------

    if item.is_null() || item.tag() == PTR_TAG_DEAD {
      return false;
    }

    // -------------------------------------------------------------------------
    // 3. Return
    // -------------------------------------------------------------------------

    true
  }

  fn do_get(&self, index: Index<'_, T>) -> Option<Arc<T>> {
    debug_assert!(
      index.get() < self.capacity() as u64,
      "ProcessTable::do_get requires that the index is in bounds",
    );

    // -------------------------------------------------------------------------
    // 1. Load Slot
    // -------------------------------------------------------------------------

    let slot: &AtomicTaggedPtr<T> = self.readonly.get_concrete(index);

    // -------------------------------------------------------------------------
    // 2. Spin
    // -------------------------------------------------------------------------

    'spin: loop {
      // -----------------------------------------------------------------------
      // 1. Load Entry
      // -----------------------------------------------------------------------

      // Acquire ensures that any prior memory operations are visible to the
      // current thread before the load. This guarantees the load is consistent
      // with other threads' modifications.
      let item: TaggedPtr<T> = slot.load(Acquire);

      // -----------------------------------------------------------------------
      // 2. Validation
      // -----------------------------------------------------------------------

      if item.is_null() || item.tag() == PTR_TAG_DEAD {
        return None;
      }

      if item.tag() == PTR_TAG_USED {
        hint::spin_loop();
        continue 'spin;
      }

      // -----------------------------------------------------------------------
      // 3. Lock Entry
      // -----------------------------------------------------------------------

      // AcqRel ensures visibility of prior memory operations, like marking the
      // entry as free. Acquire ensures that any subsequent reads will see the
      // updated state of the entry after the exchange.
      let lock: Result<TaggedPtr<T>, TaggedPtr<T>> =
        slot.compare_exchange(item, item.set(PTR_TAG_USED), AcqRel, Acquire);

      if lock.is_err() {
        hint::spin_loop();
        continue 'spin;
      }

      debug_assert!(
        slot.load(Acquire).tag() == PTR_TAG_USED,
        "ProcessTable::do_get requires that the entry is locked",
      );

      // -----------------------------------------------------------------------
      // 4. Read Entry
      // -----------------------------------------------------------------------

      debug_assert!(
        !item.is_null(),
        "ProcessTable::do_get requires that the pointer is not NULL",
      );

      debug_assert!(
        item.is_aligned(),
        "ProcessTable::do_get requires that the pointer is aligned",
      );

      // SAFETY: `ptr` is derived from a valid `Arc<T>` and assumed to be properly initialized.
      let data_source: Arc<T> = unsafe { Arc::from_raw(item.as_ptr()) };
      let data_shared: Arc<T> = Arc::clone(&data_source);

      debug_assert!(
        Arc::count(&data_shared) > 1,
        "ProcessTable::do_get requires that the refcount is > 1",
      );

      // Use `ManuallyDrop` to avoid dropping the `Arc<T>` prematurely.
      let _avoid_drop: ManuallyDrop<Arc<T>> = ManuallyDrop::new(data_source);

      // -----------------------------------------------------------------------
      // 5. Unlock Entry
      // -----------------------------------------------------------------------

      // Relaxed is used here as we are only concerned with atomicity. No
      // synchronization is needed between threads after this store.
      slot.store(item.set(PTR_TAG_FREE), Relaxed);

      // -----------------------------------------------------------------------
      // 6. Return
      // -----------------------------------------------------------------------

      break 'spin Some(data_shared);
    }
  }

  fn refresh_data(&self, pid: RawPid) -> u64 {
    let mut data: u64 = self.readonly.pid_to_base_index(pid);

    data += self.capacity() as u64;
    data &= !(!0_u64 << RawPid::PID_BITS);

    // Ensure we never stored invalid data.
    if data == RESERVED {
      data += self.capacity() as u64;
      data &= !(!0_u64 << RawPid::PID_BITS);
    }

    data
  }
}

// -----------------------------------------------------------------------------
// Process Table - Memory Allocation
// -----------------------------------------------------------------------------

impl<T> ProcessTable<T> {
  #[inline]
  const fn actual_capacity(capacity: usize) -> usize {
    if capacity < Self::MIN_ENTRIES {
      Self::MIN_ENTRIES
    } else if capacity > Self::MAX_ENTRIES {
      Self::MAX_ENTRIES
    } else {
      capacity.next_power_of_two()
    }
  }

  fn alloc_table(capacity: usize) -> Self {
    // -------------------------------------------------------------------------
    // Pointer Validation
    // -------------------------------------------------------------------------

    const {
      assert!(
        TaggedPtr::<T>::is_valid(),
        "platform pointer does not have space to store tag values",
      );
    }

    // -------------------------------------------------------------------------
    // Determine Size
    // -------------------------------------------------------------------------

    let size: usize = Self::actual_capacity(capacity);
    let bits: u32 = bit_width(size.strict_sub(1));

    // -------------------------------------------------------------------------
    // Determine Layout
    // -------------------------------------------------------------------------

    let memory: NonZeroUsize = assert_nonzero(strict_align(size.strict_mul(ENTRY_SIZE)));
    let layout: Layout = Layout::from_size_align(memory.get(), CACHE_LINE).expect("valid layout");
    let blocks: NonZeroUsize = assert_nonzero(layout.size() / CACHE_LINE);

    assert!(
      blocks.is_power_of_two(),
      "block count must be a power of two",
    );

    // -------------------------------------------------------------------------
    // Determine Format
    // -------------------------------------------------------------------------

    let id_mask_entry: u32 = 1_u32.strict_shl(bits).strict_sub(1);
    let id_mask_block: u32 = blocks.get().strict_sub(1) as u32;
    let id_mask_index: u32 = SLOT_COUNT.strict_sub(1) as u32;
    let id_shift_block: u32 = id_mask_index.trailing_ones();
    let id_shift_index: u32 = id_mask_block.trailing_ones();

    assert!(bits == id_shift_block + id_shift_index);
    assert!(id_mask_entry == (id_mask_block << id_shift_block) ^ id_mask_index);

    // -------------------------------------------------------------------------
    // Allocate
    // -------------------------------------------------------------------------

    let cap: NonZeroUsize = assert_nonzero(size);
    let ptr_concrete: NonNull<AtomicTaggedPtr<T>> = Self::alloc_concrete_array(layout, cap);
    let ptr_abstract: NonNull<AtomicU64> = Self::alloc_abstract_array(layout, blocks);

    // -------------------------------------------------------------------------
    // Handle Special Case Size
    // -------------------------------------------------------------------------

    // The table can only yield `Self::MAX_ENTRIES - 1` unique identifiers.
    //
    // To fix this without spreading complexity throughout the implementation,
    // we increment the table `len` field to permanently reserve one slot.
    //
    // The end result is a table that wastes one slot but that's not too bad...
    let len: u32 = u32::from(cap.get() == Self::MAX_ENTRIES);

    // -------------------------------------------------------------------------
    // Assemble the Table
    // -------------------------------------------------------------------------

    let this: Self = Self {
      volatile: CachePadded::new(Volatile {
        len: AtomicU32::new(len),
        aid: AtomicU64::new(0),
        fid: AtomicU64::new(0),
      }),
      readonly: CachePadded::new(ReadOnly {
        ptr_concrete,
        ptr_abstract,
        cap,
        id_mask_entry,
        id_mask_block,
        id_mask_index,
        id_shift_block,
        id_shift_index,
      }),
    };

    // -------------------------------------------------------------------------
    // Let's Go!
    // -------------------------------------------------------------------------

    this
  }

  #[inline]
  fn alloc_concrete_array(layout: Layout, count: NonZeroUsize) -> NonNull<AtomicTaggedPtr<T>> {
    assert!(layout.size() > 0, "layout size must be non-zero");

    // SAFETY: We just ensured that `layout` has a non-zero size.
    let ptr: NonNull<AtomicTaggedPtr<T>> = unsafe { Self::alloc(layout) };

    // Initialize all data atomics with a NULL ptr.
    for offset in 0..count.get() {
      // SAFETY: The iteration guarantees we are in the bounds of the allocation,
      //         therefore we satisfy all requirements of `add`. The final
      //         destination pointer is valid for writes as we've satisfied all
      //         requirements at earlier stages.
      unsafe {
        ptr.add(offset).write(AtomicTaggedPtr::null());
      }
    }

    ptr
  }

  #[inline]
  fn alloc_abstract_array(layout: Layout, blocks: NonZeroUsize) -> NonNull<AtomicU64> {
    assert!(layout.size() > 0, "layout size must be non-zero");

    // SAFETY: We just ensured that `layout` has a non-zero size.
    let ptr: NonNull<AtomicU64> = unsafe { Self::alloc(layout) };

    let mut offset: usize = 0;

    // Initialize all atomics with a pre-computed cache-line slot.
    for block in 0..blocks.get() {
      for index in 0..SLOT_COUNT {
        let value: u64 = index
          .strict_mul(blocks.get())
          .strict_add(block)
          .try_into()
          .expect("valid default slot value");

        // SAFETY: The table array is arranged as cache line-sized blocks of
        //         slots; the forumla to determine this here is `blocks * SLOT_COUNT`.
        //         Since we are guaranteed to stay in the bounds of the
        //         allocation, we satisfy all requirements of `add`. The final
        //         destination pointer is valid for writes as we've satisfied
        //         all requirements at earlier stages.
        unsafe {
          ptr.add(offset).write(AtomicU64::new(value));
        }

        offset += 1;
      }
    }

    ptr
  }

  /// Deallocate the concrete array at the given `ptr` with the given `layout`.
  ///
  /// # Safety
  ///
  /// See [`Self::dealloc`] for invariants to uphold.
  #[inline]
  unsafe fn dealloc_concrete_array(layout: Layout, ptr: NonNull<AtomicTaggedPtr<T>>) {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { Self::dealloc(layout, ptr) }
  }

  /// Deallocate the abstract array at the given `ptr` with the given `layout`.
  ///
  /// # Safety
  ///
  /// See [`Self::dealloc`] for invariants to uphold.
  #[inline]
  unsafe fn dealloc_abstract_array(layout: Layout, ptr: NonNull<AtomicU64>) {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { Self::dealloc(layout, ptr) }
  }

  /// Allocates memory as described by the given `layout`.
  ///
  /// # Safety
  ///
  /// `layout` must have a non-zero size.
  #[inline]
  unsafe fn alloc<A>(layout: Layout) -> NonNull<A> {
    // SAFETY: This is guaranteed to be safe by the caller.
    let Some(ptr) = NonNull::new(unsafe { alloc(layout) }) else {
      handle_alloc_error(layout);
    };

    ptr.cast()
  }

  /// Deallocates the block of memory at the given `ptr` with the given `layout`.
  ///
  /// # Safety
  ///
  /// The caller must ensure:
  ///
  /// - `ptr` is a block of memory currently allocated via [`Self::alloc`] and,
  ///
  /// - `layout` is the same layout that was used to allocate that block of memory.
  #[inline]
  unsafe fn dealloc<A>(layout: Layout, ptr: NonNull<A>) {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { dealloc(ptr.cast().as_ptr(), layout) }
  }
}

// -----------------------------------------------------------------------------
// Misc. Constants
// -----------------------------------------------------------------------------

/// Slot reservation marker.
const RESERVED: u64 = u64::MAX;

/// The assumed size of a cache line (in bytes).
const CACHE_LINE: usize = size_of::<CachePadded<u8>>();

/// The number of bytes required to store a table entry.
const ENTRY_SIZE: usize = {
  const INT_ALIGN: usize = align_of::<AtomicUsize>();
  const PTR_ALIGN: usize = align_of::<AtomicTaggedPtr<()>>();

  const INT_SIZE: usize = size_of::<AtomicUsize>();
  const PTR_SIZE: usize = size_of::<AtomicTaggedPtr<()>>();

  assert!(INT_ALIGN == PTR_ALIGN, "atomic pointer != atomic usize");
  assert!(INT_SIZE == PTR_SIZE, "atomic pointer != atomic usize");

  INT_SIZE
};

/// The total number of table slots available in a cache line.
const SLOT_COUNT: usize = {
  const COUNT: usize = CACHE_LINE.strict_div(ENTRY_SIZE);

  assert!(COUNT.is_power_of_two(), "slot count must be a power of two");

  COUNT
};

/// The entry is currently unlock and can be read or removed.
const PTR_TAG_FREE: u8 = 0b00;
/// the entry is currently locked for reading.
const PTR_TAG_USED: u8 = 0b01;
/// The entry is currently marked for removal.
const PTR_TAG_DEAD: u8 = 0b10;

// -----------------------------------------------------------------------------
// Misc. Utilities
// -----------------------------------------------------------------------------

// TODO: Use `usize::bit_width` when stable
#[inline]
const fn bit_width(value: usize) -> u32 {
  usize::BITS - value.leading_zeros()
}

#[inline]
const fn strict_align(value: usize) -> usize {
  value
    .strict_sub(1)
    .strict_div(CACHE_LINE)
    .strict_add(1)
    .strict_mul(CACHE_LINE)
}

#[inline]
const fn assert_nonzero(value: usize) -> NonZeroUsize {
  NonZeroUsize::new(value).expect("nonzero value")
}

// -----------------------------------------------------------------------------
// Index
// -----------------------------------------------------------------------------

/// A type storing a known-valid index into a table array.
#[repr(transparent)]
struct Index<'a, T> {
  source: u64,
  marker: PhantomData<&'a ReadOnly<T>>,
}

impl<'a, T> Index<'a, T> {
  #[inline]
  const fn new(readonly: &'a ReadOnly<T>, source: u64) -> Self {
    debug_assert!(source < readonly.cap.get() as u64);

    Self {
      source,
      marker: PhantomData,
    }
  }

  #[inline]
  const fn get(self) -> u64 {
    self.source
  }
}

impl<T> Clone for Index<'_, T> {
  #[inline]
  fn clone(&self) -> Self {
    *self
  }
}

impl<T> Copy for Index<'_, T> {}

impl<T> Debug for Index<'_, T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    write!(f, "Index({})", self.source)
  }
}

impl<T> PartialEq for Index<'_, T> {
  #[inline]
  fn eq(&self, other: &Self) -> bool {
    self.source.eq(&other.source)
  }
}

// -----------------------------------------------------------------------------
// Table State - Volatile
// -----------------------------------------------------------------------------

#[repr(C)]
struct Volatile {
  /// The total number of entries currently in the table.
  len: AtomicU32,
  /// The id counter of the next available slot to allocate.
  aid: AtomicU64,
  /// The id counter of the next available slot to free.
  fid: AtomicU64,
}

// -----------------------------------------------------------------------------
// Table State - Read-Only
// -----------------------------------------------------------------------------

#[repr(C)]
struct ReadOnly<T> {
  /// Pointer to an array of table entry data.
  ptr_concrete: NonNull<AtomicTaggedPtr<T>>,
  /// Pointer to an array mapping abstract indices to concrete indices.
  ptr_abstract: NonNull<AtomicU64>,
  /// The maximum number of entries the table can hold.
  cap: NonZeroUsize,
  /// Index mapping fields.
  id_mask_entry: u32,
  id_mask_block: u32,
  id_mask_index: u32,
  id_shift_block: u32,
  id_shift_index: u32,
}

impl<T> ReadOnly<T> {
  /// Returns a reference to the concrete array entry at `index`.
  #[inline]
  const fn get_concrete(&self, index: Index<'_, T>) -> &AtomicTaggedPtr<T> {
    // SAFETY: The `Index` type is known to be in bounds.
    unsafe { self.concrete_unchecked(index.get() as usize) }
  }

  /// Returns a reference to the abstract array entry at `index`.
  #[inline]
  const fn get_abstract(&self, index: Index<'_, T>) -> &AtomicU64 {
    // SAFETY: The `Index` type is known to be in bounds.
    unsafe { self.abstract_unchecked(index.get() as usize) }
  }

  /// Returns a reference to the concrete array entry at `index`,
  /// without performing any safety checks.
  ///
  /// # Safety
  ///
  /// - `index` must be *less* than the value of [`cap`][Self::cap] and,
  ///
  /// - the pointer to the entry at `index` must be [convertible to a reference].
  ///
  /// [convertible to a reference]: std::ptr#pointer-to-reference-conversion
  #[inline]
  const unsafe fn concrete_unchecked(&self, index: usize) -> &AtomicTaggedPtr<T> {
    #[cfg(debug_assertions)]
    if index >= self.cap.get() {
      panic!("ReadOnly::concrete_unchecked requires that the index is in bounds");
    }

    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.ptr_concrete.add(index).as_ref() }
  }

  /// Returns a reference to the abstract array entry at `index`,
  /// without performing any safety checks.
  ///
  /// # Safety
  ///
  /// - `index` must be *less* than the value of [`cap`][Self::cap] and,
  ///
  /// - the pointer to the entry at `index` must be [convertible to a reference].
  ///
  /// [convertible to a reference]: std::ptr#pointer-to-reference-conversion
  #[inline]
  const unsafe fn abstract_unchecked(&self, index: usize) -> &AtomicU64 {
    #[cfg(debug_assertions)]
    if index >= self.cap.get() {
      panic!("ReadOnly::abstract_unchecked requires that the index is in bounds");
    }

    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.ptr_abstract.add(index).as_ref() }
  }

  // ---------------------------------------------------------------------------
  // Index Mapping
  //
  // The following functions are based on the Erlang/OTP Implementation
  //
  // pid_data_to_base_index   -> erts_ptab_pixdata2data
  // pid_data_to_real_index   -> erts_ptab_pixdata2pix
  // base_index_to_real_index -> erts_ptab_data2pix
  // base_index_to_pid_data   -> erts_ptab_data2pixdata
  // base_index_to_pid        -> erts_ptab_make_id
  // pid_to_real_index        -> erts_ptab_id2pix
  // pid_to_base_index        -> erts_ptab_id2data
  // ---------------------------------------------------------------------------

  #[inline]
  const fn pid_data_to_base_index(&self, input: u64) -> u64 {
    let mut value: u64 = input & !(self.id_mask_entry as u64);

    value |= (input >> self.id_shift_block) & self.id_mask_block as u64;
    value |= (input & self.id_mask_index as u64) << self.id_shift_index;

    value
  }

  #[inline]
  const fn pid_data_to_real_index(&self, input: u64) -> Index<'_, T> {
    Index::new(self, input & self.id_mask_entry as u64)
  }

  #[inline]
  const fn base_index_to_real_index(&self, input: u64) -> Index<'_, T> {
    let mut value: u64 = 0;

    value += (input & self.id_mask_block as u64) << self.id_shift_block;
    value += (input >> self.id_shift_index) & self.id_mask_index as u64;

    Index::new(self, value)
  }

  #[inline]
  const fn base_index_to_pid_data(&self, input: u64) -> u64 {
    let value: u64 = input & !(self.id_mask_entry as u64);
    let value: u64 = value | self.base_index_to_real_index(input).get();

    debug_assert!(self.pid_data_to_base_index(value) == input);

    value
  }

  #[inline]
  const fn base_index_to_pid(&self, input: u64) -> RawPid {
    let value: u64 = input & ((1_u64 << RawPid::PID_BITS) - 1);
    let value: u64 = self.base_index_to_pid_data(value);
    let value: u64 = (value << RawPid::TAG_BITS) | RawPid::TAG_DATA;

    RawPid::from_bits(value)
  }

  #[inline]
  const fn pid_to_real_index(&self, input: RawPid) -> Index<'_, T> {
    let value: u64 = input.into_bits() >> RawPid::TAG_BITS;
    let value: Index<'_, T> = self.pid_data_to_real_index(value);

    value
  }

  #[inline]
  const fn pid_to_base_index(&self, input: RawPid) -> u64 {
    let value: u64 = input.into_bits() >> RawPid::TAG_BITS;
    let value: u64 = self.pid_data_to_base_index(value);

    value
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  type Table = ProcessTable<usize>;

  fn write(value: usize) -> impl Fn(&mut MaybeUninit<usize>, RawPid) {
    move |ptr, _pid| {
      ptr.write(value);
    }
  }

  #[test]
  fn test_capacity_min() {
    let table: Table = Table::with_capacity(usize::MIN);
    assert_eq!(table.capacity(), Table::MIN_ENTRIES);
  }

  #[test]
  fn test_capacity_max() {
    let table: Table = Table::with_capacity(usize::MAX);
    assert_eq!(table.capacity(), Table::MAX_ENTRIES);
  }

  #[test]
  fn test_actual_capacity() {
    let min: usize = Table::MIN_ENTRIES;
    let max: usize = Table::MAX_ENTRIES;

    for capacity in min..=max {
      assert_eq!(
        Table::actual_capacity(capacity),
        capacity.next_power_of_two()
      );
    }
  }

  #[test]
  fn test_id_mapping() {
    let table: Table = Table::new();

    for index in 0..table.capacity() * 3 {
      let real: Index<'_, usize> = table.readonly.base_index_to_real_index(index as u64);
      let data: u64 = table.readonly.base_index_to_pid_data(index as u64);
      let rpid: RawPid = table.readonly.base_index_to_pid(index as u64);

      assert_eq!(table.readonly.pid_data_to_base_index(data), index as u64);
      assert_eq!(table.readonly.pid_data_to_real_index(data), real);

      assert_eq!(table.readonly.pid_to_real_index(rpid), real);
      assert_eq!(table.readonly.pid_to_base_index(rpid), index as u64);
    }
  }

  #[test]
  fn test_new_table() {
    let table: Table = Table::with_capacity(1 << 6);

    assert_eq!(table.capacity(), 1 << 6);
    assert_eq!(table.len(), 0);
    assert_eq!(table.is_empty(), true);
  }

  #[test]
  fn test_is_empty() {
    let table: Table = Table::new();

    assert_eq!(table.is_empty(), true);
    table.insert(write(10)).unwrap();
    assert_eq!(table.is_empty(), false);
  }

  #[test]
  fn test_insert() {
    let table: Table = Table::new();

    let _item: Arc<usize> = table.insert(write(1)).unwrap();
    let _item: Arc<usize> = table.insert(write(2)).unwrap();

    assert_eq!(table.len(), 2);
    assert_eq!(table.is_empty(), false);
  }

  #[test]
  fn test_keys() {
    let table: Table = Table::new();

    let _item: Arc<usize> = table.insert(write(1)).unwrap();
    let _item: Arc<usize> = table.insert(write(2)).unwrap();

    let keys: Vec<RawPid> = table.keys().collect();

    assert_eq!(keys.len(), 2);
  }
}
