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
//! 2. **Real indices**: Physical memory locations in cache-line-aligned blocks
//!
//! This separation enables lock-free allocation while maintaining cache efficiency.
//!
//! # Slot States
//!
//! Each slot is a tagged pointer with three states:
//!
//! - `FREE` (0b00): Slot contains valid data and can be accessed
//! - `USED` (0b01): Slot is temporarily locked during read/remove operations
//! - `DEAD` (0b10): Slot has been logically removed
//!
//! # Concurrency Model
//!
//! - **Insertion**: Lock-free via atomic counter and compare-exchange
//! - **Removal**: Lock-free with spinlock on individual slots
//! - **Lookup**: Lock-free with spinlock on individual slots
//! - **Iteration**: Snapshot-based, not affected by concurrent modifications
//!
//! # Memory Layout
//!
//! Entries are organized into cache-line-sized blocks to reduce contention:
//!
//! ```text
//! ┌───────────┐
//! │ 0 4  8 12 │ Cache Line (Block 0)
//! ├───────────┤
//! │ 1 5  9 13 │ Cache Line (Block 1)
//! ├───────────┤
//! │ 2 6 10 14 │ Cache Line (Block 2)
//! ├───────────┤
//! │ 3 7 11 15 │ Cache Line (Block 3)
//! └───────────┘
//! ```
//!
//! Sequential allocation spreads entries across blocks, with each block
//! residing in its own cache line.

use crossbeam_utils::CachePadded;
use std::alloc::handle_alloc_error;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::num::NonZeroUsize;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;
use std::ptr::NonNull;
use triomphe::Arc;
use triomphe::UniqueArc;

use crate::core::InternalPid;
use crate::loom::alloc::Layout;
use crate::loom::alloc::alloc;
use crate::loom::alloc::dealloc;
use crate::loom::hint;
use crate::loom::sync::atomic::AtomicU32;
use crate::loom::sync::atomic::AtomicU64;
use crate::loom::sync::atomic::AtomicUsize;
use crate::loom::sync::atomic::Ordering::AcqRel;
use crate::loom::sync::atomic::Ordering::Acquire;
use crate::loom::sync::atomic::Ordering::Relaxed;
use crate::loom::sync::atomic::Ordering::Release;
use crate::tyre::ptr::AtomicTaggedPtr;
use crate::tyre::ptr::TaggedPtr;

// -----------------------------------------------------------------------------
// Table Full Error
// -----------------------------------------------------------------------------

/// Error returned when the process table has reached its capacity.
///
/// This error occurs when attempting to insert a process into a full table.
/// The table capacity is fixed at creation time and cannot be expanded.
#[derive(Debug)]
#[non_exhaustive]
pub struct ProcTableFull;

impl Display for ProcTableFull {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("too many processes")
  }
}

impl Error for ProcTableFull {}

// -----------------------------------------------------------------------------
// Table Keys Iterator
// -----------------------------------------------------------------------------

/// Iterator over the PIDs of currently allocated processes in a [`ProcTable`].
///
/// This iterator provides a snapshot view of allocated PIDs at the time
/// iteration begins. Concurrent insertions or removals do not invalidate
/// the iterator, though yielded PIDs may refer to processes that have
/// since been removed.
pub struct ProcTableKeys<'a, T> {
  table: &'a ProcTable<T>,
  index: usize,
}

impl<'a, T> ProcTableKeys<'a, T> {
  #[inline]
  const fn new(table: &'a ProcTable<T>) -> Self {
    Self { table, index: 0 }
  }
}

impl<T> Debug for ProcTableKeys<'_, T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("ProcTableKeys(..)")
  }
}

impl<'a, T> Iterator for ProcTableKeys<'a, T> {
  type Item = InternalPid;

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

/// Lock-free, cache-line-aware process table storing [`Arc<T>`] entries.
///
/// This table provides concurrent insertion, removal, and lookup of process
/// entries without traditional locks. Each operation uses atomic primitives
/// and tagged pointers to ensure thread safety.
///
/// # Capacity
///
/// Table capacity is fixed at creation time and must be between
/// [`MIN_ENTRIES`] and [`MAX_ENTRIES`]. Capacity is automatically rounded
/// to the next power of two.
///
/// # Thread Safety
///
/// All operations are safe to call concurrently from multiple threads:
///
/// - **Readers** never block writers or other readers
/// - **Writers** coordinate only on the specific slot being modified
/// - **No global locks** are used for normal operations
///
/// # Memory Ordering
///
/// The table uses specific atomic orderings for different operations:
///
/// - **Relaxed**: Used for counters where only atomicity matters (no cross-thread visibility needed)
/// - **Acquire**: Loads that need to see all prior writes to the slot
/// - **Release**: Stores that make all prior writes visible to other threads
/// - **AcqRel**: Operations that both load previous state and make writes visible
///
/// # Examples
///
/// ```
/// use errai::core::ProcTable;
///
/// let table = ProcTable::<u64>::with_capacity(1024);
///
/// // Check capacity
/// assert!(table.capacity() >= 1024);
/// assert!(table.is_empty());
/// ```
///
/// [`MIN_ENTRIES`]: Self::MIN_ENTRIES
/// [`MAX_ENTRIES`]: Self::MAX_ENTRIES
#[repr(C)]
pub struct ProcTable<T> {
  volatile: CachePadded<Volatile>,
  readonly: CachePadded<ReadOnly<T>>,
}

impl<T> ProcTable<T> {
  /// Minimum number of entries supported in the table.
  pub const MIN_ENTRIES: usize = 1 << 4;

  /// Maximum number of entries supported in the table.
  pub const MAX_ENTRIES: usize = {
    #[cfg(not(any(miri, test)))]
    {
      1 << 27
    }
    #[cfg(any(miri, test))]
    {
      1 << 20
    }
  };

  /// Default number of entries in a new table.
  pub const DEF_ENTRIES: usize = {
    #[cfg(not(any(miri, test)))]
    {
      1 << 20
    }
    #[cfg(any(miri, test))]
    {
      1 << 10
    }
  };

  /// Creates a new, empty table using the default capacity.
  #[inline]
  pub fn new() -> Self {
    Self::with_capacity(Self::DEF_ENTRIES)
  }

  /// Creates a new, empty table with at least `capacity` slots.
  ///
  /// The actual capacity will be rounded up to the next power of two and
  /// clamped between [`MIN_ENTRIES`] and [`MAX_ENTRIES`].
  ///
  /// # Examples
  ///
  /// ```
  /// use errai::core::ProcTable;
  ///
  /// let table = ProcTable::<u64>::with_capacity(1000);
  /// assert_eq!(table.capacity(), 1024);  // Rounded to power of two
  /// ```
  ///
  /// [`MIN_ENTRIES`]: Self::MIN_ENTRIES
  /// [`MAX_ENTRIES`]: Self::MAX_ENTRIES
  pub fn with_capacity(capacity: usize) -> Self {
    Self::alloc_table(capacity)
  }

  /// Returns the maximum number of entries that the table can hold.
  #[inline]
  pub fn capacity(&self) -> usize {
    self.readonly.cap.get()
  }

  /// Returns the total number of entries currently allocated in the table.
  ///
  /// This value may change immediately after reading due to concurrent
  /// operations in other threads.
  #[inline]
  pub fn len(&self) -> usize {
    self.volatile.len.load(Relaxed) as usize
  }

  /// Returns `true` if the table currently contains no entries.
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  /// Inserts a new entry into the table and returns a shared [`Arc<T>`].
  ///
  /// The `init` function is called with a [`MaybeUninit<T>`] and the assigned
  /// PID. It must fully initialize the value before returning.
  ///
  /// # Requirements for `init`
  ///
  /// - **Must** fully initialize the [`MaybeUninit<T>`] before returning
  /// - **Must not** panic (panics may permanently leak a slot)
  /// - **Should** avoid heavy computation or recursive table operations
  ///
  /// # Errors
  ///
  /// Returns [`ProcTableFull`] if the table has reached maximum capacity.
  #[inline]
  pub fn insert<F>(&self, init: F) -> Result<Arc<T>, ProcTableFull>
  where
    F: FnOnce(&mut MaybeUninit<T>, InternalPid),
  {
    self.do_insert(init)
  }

  /// Removes the entry associated with `pid` and returns it if present.
  ///
  /// The slot becomes available for reuse after removal. If multiple threads
  /// attempt to remove the same PID, only one will succeed.
  pub fn remove(&self, pid: InternalPid) -> Option<Arc<T>> {
    self.do_remove(pid, self.readonly.pid_to_real_index(pid))
  }

  /// Returns `true` if an entry exists for the given `pid`.
  ///
  /// The result may be stale immediately after returning due to concurrent
  /// operations.
  #[inline]
  pub fn exists(&self, pid: InternalPid) -> bool {
    self.do_exists(self.readonly.pid_to_real_index(pid))
  }

  /// Returns the entry associated with `pid`, if it exists.
  ///
  /// The returned [`Arc<T>`] can be safely shared across threads.
  #[inline]
  pub fn get(&self, pid: InternalPid) -> Option<Arc<T>> {
    self.do_get(self.readonly.pid_to_real_index(pid))
  }

  /// Returns `true` if a slot exists at the given base `index`.
  #[inline]
  pub fn has(&self, index: usize) -> bool {
    if index < self.capacity() {
      self.do_exists(self.readonly.base_index_to_real_index(index as u64))
    } else {
      false
    }
  }

  /// Returns an iterator over all currently allocated PIDs.
  ///
  /// The iterator provides a snapshot view and is not invalidated by
  /// concurrent modifications.
  #[inline]
  pub fn keys(&self) -> ProcTableKeys<'_, T> {
    ProcTableKeys::new(self)
  }

  /// Translates a PID into its `(number, serial)` components.
  ///
  /// This is useful for displaying PIDs or integrating with external systems.
  #[inline]
  pub fn translate_pid(&self, pid: InternalPid) -> (u32, u32) {
    fn bits(value: u64, start: u32, count: u32) -> u64 {
      (value >> start) & !(!0_u64 << count)
    }

    let source: u64 = self.readonly.pid_to_base_index(pid);
    let number: u32 = bits(source, 0, InternalPid::NUMBER_BITS) as u32;
    let serial: u32 = bits(source, InternalPid::NUMBER_BITS, InternalPid::SERIAL_BITS) as u32;

    (number, serial)
  }
}

// -----------------------------------------------------------------------------
// Process Table - Internals
// -----------------------------------------------------------------------------

unsafe impl<T: Send> Send for ProcTable<T> {}
unsafe impl<T: Send> Sync for ProcTable<T> {}

impl<T> RefUnwindSafe for ProcTable<T> {}
impl<T> UnwindSafe for ProcTable<T> {}

impl<T> Drop for ProcTable<T> {
  fn drop(&mut self) {
    let memory: usize = strict_align(self.capacity().strict_mul(ENTRY_SIZE));
    let layout: Layout = Layout::from_size_align(memory, CACHE_LINE).expect("valid layout");

    // SAFETY: Both pointers were allocated by `alloc_table` using this layout.
    //         We're the sole owner at drop time, so deallocation is safe.
    unsafe {
      Self::dealloc_concrete_array(layout, self.readonly.ptr_concrete);
      Self::dealloc_abstract_array(layout, self.readonly.ptr_abstract);
    }
  }
}

impl<T> Debug for ProcTable<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    struct Mask(u32);

    impl Debug for Mask {
      fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{:0>32b}", &self.0)
      }
    }

    f.debug_struct("ProcTable")
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

impl<T> ProcTable<T> {
  fn do_insert<F>(&self, init: F) -> Result<Arc<T>, ProcTableFull>
  where
    F: FnOnce(&mut MaybeUninit<T>, InternalPid),
  {
    // -------------------------------------------------------------------------
    // 1. Reserve Slot
    // -------------------------------------------------------------------------

    'reserve: {
      // Relaxed: We only need atomicity for the increment, not cross-thread
      // visibility. The capacity check doesn't require synchronization.
      let count: u32 = self.volatile.len.fetch_add(1, Relaxed);

      if count < self.capacity() as u32 {
        break 'reserve;
      }

      let mut count: u32 = count + 1;

      // Relaxed: If we exceed capacity, we undo the increment. We only need
      // atomicity for the compare-exchange, not memory ordering.
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

      return Err(ProcTableFull);
    }

    // -------------------------------------------------------------------------
    // 2. Allocate Entry
    // -------------------------------------------------------------------------

    let mut uninit: UniqueArc<MaybeUninit<T>> = UniqueArc::new_uninit();

    // -------------------------------------------------------------------------
    // 3. Load Slot Index
    // -------------------------------------------------------------------------

    let base_index: u64 = 'acquire: loop {
      // Relaxed: We only need atomicity for getting the next slot index.
      // The reservation system ensures we don't collide.
      let base_index: u64 = self.volatile.aid.fetch_add(1, Relaxed);
      let real_index: Index<'_, T> = self.readonly.base_index_to_real_index(base_index);

      // AcqRel: We need to see if the slot is reserved (Acquire) and make
      // our reservation visible to others (Release).
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
      "ProcTable::do_insert requires that the index is in bounds",
    );

    // -------------------------------------------------------------------------
    // 4. Initialize Entry
    // -------------------------------------------------------------------------

    init(&mut uninit, self.readonly.base_index_to_pid(base_index));

    // -------------------------------------------------------------------------
    // 5. Write Entry
    // -------------------------------------------------------------------------

    // SAFETY: `uninit` was initialized by the provided `init` function,
    //         which is required to fully initialize the value.
    let data_unique: UniqueArc<T> = unsafe { UniqueArc::assume_init(uninit) };
    let data_shared: Arc<T> = UniqueArc::shareable(data_unique);

    let slot: &AtomicTaggedPtr<T> = self.readonly.get_concrete(real_index);
    let heap: *mut T = Arc::into_raw(Arc::clone(&data_shared)).cast_mut();

    // SAFETY: Pointer alignment was verified during table setup.
    let item: TaggedPtr<T> = unsafe { TaggedPtr::with_unchecked(heap, PTR_TAG_FREE) };

    debug_assert!(
      !item.is_null(),
      "ProcTable::do_insert requires that the pointer is not NULL",
    );

    debug_assert!(
      item.is_aligned(),
      "ProcTable::do_get requires that the pointer is aligned",
    );

    // Release: Make the initialized entry visible to all other threads.
    // Anyone who Acquires this slot will see the fully initialized data.
    slot.store(item, Release);

    // -------------------------------------------------------------------------
    // 6. Return
    // -------------------------------------------------------------------------

    Ok(data_shared)
  }

  fn do_remove(&self, pid: InternalPid, index: Index<'_, T>) -> Option<Arc<T>> {
    debug_assert!(
      index.get() < self.capacity() as u64,
      "ProcTable::do_remove requires that the index is in bounds",
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

      // Acquire: Ensure we see the latest state of the slot.
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

      // AcqRel: See the current FREE state (Acquire) and make our lock visible (Release).
      let lock: Result<TaggedPtr<T>, TaggedPtr<T>> =
        slot.compare_exchange(item, item.set(PTR_TAG_USED), AcqRel, Acquire);

      if lock.is_err() {
        hint::spin_loop();
        continue 'spin;
      }

      debug_assert!(
        slot.load(Acquire).tag() == PTR_TAG_USED,
        "ProcTable::do_remove requires that the entry is locked",
      );

      // -----------------------------------------------------------------------
      // 4. Read Entry
      // -----------------------------------------------------------------------

      debug_assert!(
        !item.is_null(),
        "ProcTable::do_remove requires that the pointer is not NULL",
      );

      debug_assert!(
        item.is_aligned(),
        "ProcTable::do_remove requires that the pointer is aligned",
      );

      // SAFETY: The pointer was created from a valid Arc during insertion.
      let data_source: Arc<T> = unsafe { Arc::from_raw(item.as_ptr()) };

      // -----------------------------------------------------------------------
      // 5. Delete Entry
      // -----------------------------------------------------------------------

      // Release: Make the null write visible to all threads.
      slot.store(TaggedPtr::null(), Release);

      // -----------------------------------------------------------------------
      // 6. Release Slot
      // -----------------------------------------------------------------------

      let data: u64 = self.refresh_data(pid);

      debug_assert!(
        data != RESERVED,
        "ProcTable::do_remove requires that the slot data is valid",
      );

      debug_assert!(
        index == self.readonly.base_index_to_real_index(data),
        "ProcTable::do_remove requires that the slot mapping is reversible",
      );

      'release: loop {
        // Relaxed: We only need atomicity for claiming the next free slot.
        let base_index: u64 = self.volatile.fid.fetch_add(1, Relaxed);
        let real_index: Index<'_, T> = self.readonly.base_index_to_real_index(base_index);

        // Relaxed: We're just updating the free list, no synchronization needed.
        let atomic: &AtomicU64 = self.readonly.get_abstract(real_index);
        let result: Result<u64, u64> = atomic.compare_exchange(RESERVED, data, Relaxed, Relaxed);

        if result.is_ok() {
          break 'release;
        }
      }

      debug_assert!(
        self.len() > 0,
        "ProcTable::do_remove requires that length is > 0",
      );

      // Release: Make the length decrement visible to all threads.
      let _update_len: u32 = self.volatile.len.fetch_sub(1, Release);

      // -----------------------------------------------------------------------
      // 7. Return
      // -----------------------------------------------------------------------

      break 'spin Some(data_source);
    }
  }

  fn do_exists(&self, index: Index<'_, T>) -> bool {
    debug_assert!(
      index.get() < self.capacity() as u64,
      "ProcTable::do_exists requires that the index is in bounds",
    );

    // -------------------------------------------------------------------------
    // 1. Load Slot/Entry
    // -------------------------------------------------------------------------

    // Acquire: Ensure we see the latest state of the slot.
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
      "ProcTable::do_get requires that the index is in bounds",
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

      // Acquire: Ensure we see the latest slot state.
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

      // AcqRel: See the current FREE state (Acquire) and make our lock visible (Release).
      let lock: Result<TaggedPtr<T>, TaggedPtr<T>> =
        slot.compare_exchange(item, item.set(PTR_TAG_USED), AcqRel, Acquire);

      if lock.is_err() {
        hint::spin_loop();
        continue 'spin;
      }

      debug_assert!(
        slot.load(Acquire).tag() == PTR_TAG_USED,
        "ProcTable::do_get requires that the entry is locked",
      );

      // -----------------------------------------------------------------------
      // 4. Read Entry
      // -----------------------------------------------------------------------

      debug_assert!(
        !item.is_null(),
        "ProcTable::do_get requires that the pointer is not NULL",
      );

      debug_assert!(
        item.is_aligned(),
        "ProcTable::do_get requires that the pointer is aligned",
      );

      // SAFETY: The pointer was created from a valid Arc during insertion.
      let data_source: Arc<T> = unsafe { Arc::from_raw(item.as_ptr()) };
      let data_shared: Arc<T> = Arc::clone(&data_source);

      debug_assert!(
        Arc::count(&data_shared) > 1,
        "ProcTable::do_get requires that the refcount is > 1",
      );

      // Prevent dropping the Arc we just reconstructed, as the table still owns it.
      let _avoid_drop: ManuallyDrop<Arc<T>> = ManuallyDrop::new(data_source);

      // -----------------------------------------------------------------------
      // 5. Unlock Entry
      // -----------------------------------------------------------------------

      // Relaxed: We're just clearing the lock bit. The Arc clone is already done.
      slot.store(item.set(PTR_TAG_FREE), Relaxed);

      // -----------------------------------------------------------------------
      // 6. Return
      // -----------------------------------------------------------------------

      break 'spin Some(data_shared);
    }
  }

  fn refresh_data(&self, pid: InternalPid) -> u64 {
    let mut data: u64 = self.readonly.pid_to_base_index(pid);

    data += self.capacity() as u64;
    data &= !(!0_u64 << InternalPid::PID_BITS);

    // Ensure we never store the reserved marker value.
    if data == RESERVED {
      data += self.capacity() as u64;
      data &= !(!0_u64 << InternalPid::PID_BITS);
    }

    data
  }
}

// -----------------------------------------------------------------------------
// Process Table - Memory Allocation
// -----------------------------------------------------------------------------

impl<T> ProcTable<T> {
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
    // The end result is a table that wastes one slot but simplifies the code.
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

    // SAFETY: Layout has non-zero size, verified above.
    let ptr: NonNull<AtomicTaggedPtr<T>> = unsafe { Self::alloc(layout) };

    // Initialize all slots with null pointers.
    for offset in 0..count.get() {
      // SAFETY: The offset is within bounds of the allocation (0..count).
      //         The pointer is properly aligned and valid for writes.
      unsafe {
        ptr.add(offset).write(AtomicTaggedPtr::null());
      }
    }

    ptr
  }

  #[inline]
  fn alloc_abstract_array(layout: Layout, blocks: NonZeroUsize) -> NonNull<AtomicU64> {
    assert!(layout.size() > 0, "layout size must be non-zero");

    // SAFETY: Layout has non-zero size, verified above.
    let ptr: NonNull<AtomicU64> = unsafe { Self::alloc(layout) };

    let mut offset: usize = 0;

    // Initialize the abstract index mapping with cache-line-aware layout.
    for block in 0..blocks.get() {
      for index in 0..SLOT_COUNT {
        let value: u64 = index
          .strict_mul(blocks.get())
          .strict_add(block)
          .try_into()
          .expect("valid default slot value");

        // SAFETY: Total slots = blocks * SLOT_COUNT. We iterate exactly that many
        //         times, so offset stays in bounds. Pointer is aligned and valid.
        unsafe {
          ptr.add(offset).write(AtomicU64::new(value));
        }

        offset += 1;
      }
    }

    ptr
  }

  /// Deallocate the concrete array.
  ///
  /// # Safety
  ///
  /// - `ptr` must have been allocated via [`alloc_concrete_array`]
  /// - `layout` must match the layout used during allocation
  ///
  /// [`alloc_concrete_array`]: Self::alloc_concrete_array
  #[inline]
  unsafe fn dealloc_concrete_array(layout: Layout, ptr: NonNull<AtomicTaggedPtr<T>>) {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { Self::dealloc(layout, ptr) }
  }

  /// Deallocate the abstract array.
  ///
  /// # Safety
  ///
  /// - `ptr` must have been allocated via [`alloc_abstract_array`]
  /// - `layout` must match the layout used during allocation
  ///
  /// [`alloc_abstract_array`]: Self::alloc_abstract_array
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

  /// Deallocates the block of memory at the given `ptr`.
  ///
  /// # Safety
  ///
  /// - `ptr` must have been allocated via [`Self::alloc`] with `layout`
  /// - `layout` must be the same layout used during allocation
  /// - `ptr` must not be used after this call
  #[inline]
  unsafe fn dealloc<A>(layout: Layout, ptr: NonNull<A>) {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { dealloc(ptr.cast().as_ptr(), layout) }
  }
}

// -----------------------------------------------------------------------------
// Misc. Constants
// -----------------------------------------------------------------------------

/// Slot reservation marker indicating the slot is being allocated or freed.
const RESERVED: u64 = u64::MAX;

/// The assumed size of a cache line in bytes.
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

/// Tag indicating the slot is unlocked and contains valid data.
const PTR_TAG_FREE: u8 = 0b00;
/// Tag indicating the slot is temporarily locked during an operation.
const PTR_TAG_USED: u8 = 0b01;
/// Tag indicating the slot has been logically removed.
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

/// A type-safe index into the table arrays.
///
/// This type ensures indices are always valid through phantom lifetime.
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

/// Frequently modified table state stored in a cache-padded section.
#[repr(C)]
struct Volatile {
  /// The total number of entries currently in the table.
  len: AtomicU32,
  /// Allocation counter for the next slot to allocate.
  aid: AtomicU64,
  /// Free counter for the next slot to return to the free list.
  fid: AtomicU64,
}

// -----------------------------------------------------------------------------
// Table State - Read-Only
// -----------------------------------------------------------------------------

/// Static table metadata stored in a cache-padded section.
///
/// This data is initialized once and never modified, avoiding cache
/// line bouncing between threads.
#[repr(C)]
struct ReadOnly<T> {
  /// Pointer to the array of table entries (concrete data).
  ptr_concrete: NonNull<AtomicTaggedPtr<T>>,
  /// Pointer to the array mapping abstract to concrete indices.
  ptr_abstract: NonNull<AtomicU64>,
  /// The maximum number of entries the table can hold.
  cap: NonZeroUsize,
  /// Index mapping masks and shifts for PID translation.
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

  /// Returns a reference to the concrete array entry without bounds checking.
  ///
  /// # Safety
  ///
  /// - `index` must be less than `cap`
  /// - The pointer at `index` must be properly initialized and aligned
  #[inline]
  const unsafe fn concrete_unchecked(&self, index: usize) -> &AtomicTaggedPtr<T> {
    #[cfg(debug_assertions)]
    if index >= self.cap.get() {
      panic!("ReadOnly::concrete_unchecked requires that the index is in bounds");
    }

    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.ptr_concrete.add(index).as_ref() }
  }

  /// Returns a reference to the abstract array entry without bounds checking.
  ///
  /// # Safety
  ///
  /// - `index` must be less than `cap`
  /// - The pointer at `index` must be properly initialized and aligned
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
  const fn base_index_to_pid(&self, input: u64) -> InternalPid {
    let value: u64 = input & ((1_u64 << InternalPid::PID_BITS) - 1);
    let value: u64 = self.base_index_to_pid_data(value);
    let value: u64 = (value << InternalPid::TAG_BITS) | InternalPid::TAG_DATA;

    InternalPid::from_bits(value)
  }

  #[inline]
  const fn pid_to_real_index(&self, input: InternalPid) -> Index<'_, T> {
    let value: u64 = input.into_bits() >> InternalPid::TAG_BITS;
    let value: Index<'_, T> = self.pid_data_to_real_index(value);

    value
  }

  #[inline]
  const fn pid_to_base_index(&self, input: InternalPid) -> u64 {
    let value: u64 = input.into_bits() >> InternalPid::TAG_BITS;
    let value: u64 = self.pid_data_to_base_index(value);

    value
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use std::mem::MaybeUninit;
  use std::thread;
  use std::thread::JoinHandle;
  use triomphe::Arc;

  use crate::core::InternalPid;
  use crate::core::table::ProcTable;
  use crate::core::table::proc_table::Index;

  type Table = ProcTable<usize>;

  fn write(value: usize) -> impl Fn(&mut MaybeUninit<usize>, InternalPid) {
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
      let rpid: InternalPid = table.readonly.base_index_to_pid(index as u64);

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

    let keys: Vec<InternalPid> = table.keys().collect();

    assert_eq!(keys.len(), 2);
  }

  #[cfg(not(miri))]
  #[test]
  fn test_concurrent_stress() {
    const THREAD_NUM: usize = 100;
    const THREAD_OPS: usize = 1000;

    let table: Arc<Table> = Arc::new(Table::with_capacity(1024));
    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    for i in 0..THREAD_NUM {
      let table: Arc<Table> = Arc::clone(&table);

      let handle: JoinHandle<()> = thread::spawn(move || {
        for j in 0..THREAD_OPS {
          let mut new_pid: Option<InternalPid> = None;

          let result = table.insert(|val, pid| {
            new_pid = Some(pid);
            val.write(i * THREAD_OPS + j);
          });

          if let Ok(_arc) = result {
            let pid = new_pid.unwrap();
            let removed = table.remove(pid);

            assert_eq!(removed.map(|item| *item), Some(i * THREAD_OPS + j));
          }
        }
      });

      handles.push(handle);
    }

    for handle in handles {
      handle.join().unwrap();
    }

    assert_eq!(table.len(), 0);
  }
}
