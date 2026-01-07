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
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::mem::MaybeUninit;
use std::num::NonZeroUsize;
use std::panic::RefUnwindSafe;
use std::panic::UnwindSafe;
use std::ptr;
use std::ptr::NonNull;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use triomphe::Arc;
use triomphe::UniqueArc;

use crate::lang::RawPid;

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
  const PTR_ALIGN: usize = align_of::<AtomicPtr<()>>();

  const INT_SIZE: usize = size_of::<AtomicUsize>();
  const PTR_SIZE: usize = size_of::<AtomicPtr<()>>();

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
// Permit
// -----------------------------------------------------------------------------

/// A permit to add one entry to the table.
#[repr(transparent)]
struct Permit<'a, T>(PhantomData<&'a ProcessTable<T>>);

impl<'a, T> Permit<'a, T> {
  #[inline]
  const fn new() -> Self {
    Self(PhantomData)
  }
}

// -----------------------------------------------------------------------------
// Table Full Error
// -----------------------------------------------------------------------------

#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct ProcessTableFull;

impl Display for ProcessTableFull {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str("Too many processes")
  }
}

impl Error for ProcessTableFull {}

// -----------------------------------------------------------------------------
// Table Iterator
// -----------------------------------------------------------------------------

pub struct Iter<'a, T> {
  table: &'a ProcessTable<T>,
  index: usize,
}

impl<'a, T> Iter<'a, T> {
  #[inline]
  const fn new(table: &'a ProcessTable<T>) -> Self {
    Self { table, index: 0 }
  }
}

impl<'a, T> Iterator for Iter<'a, T> {
  type Item = Arc<T>;

  fn next(&mut self) -> Option<Self::Item> {
    loop {
      if self.table.capacity() < self.index {
        return None;
      }

      if let Some(item) = self.table.at(self.index) {
        self.index += 1;
        return Some(item);
      }

      self.index += 1;
    }
  }
}

// -----------------------------------------------------------------------------
// Process Table
// -----------------------------------------------------------------------------

/// Cache Line Aware Process Table
///
/// The process table is a lock-free structure designed to store a large number
/// of entries with an emphasis on cache line awareness, enabling efficient
/// concurrent access to table entries.
///
/// The core approach is to divide the table into cache line sized **blocks**,
/// where each block has a fixed number of **slots** used to store atomic
/// pointers. As entries are inserted into the table, they are placed in
/// subsequent blocks. When the *block index* reaches the end, the *block index*
/// wraps and the *slot index* is incremented.
///
/// ## Memory Structure
///
/// Below is a visual overview of the mapping from **index** to **slot**.
///
/// ```text
/// ┌──────────┐┌──────────┐┌───────────┐┌───────────┐
/// │ 0 4 8 12 ││ 1 5 9 13 ││ 2 6 10 14 ││ 3 7 11 15 │
/// └──────────┘└──────────┘└───────────┘└───────────┘
/// ```
///
/// ## Internal Structure
///
/// The table is split into two sections, each aligned to the length of a cache line.
///
/// ### Volatile Section
///
/// Contains atomic integers used for starvation-resistant slot allocation.
/// These integers are frequently accessed concurrently by multiple threads
/// during insertions and removals.
///
/// ### Read-Only Section
///
/// Holds static data for the lifetime of the table, including layout metadata
/// and pointers to various allocations.
#[repr(C)]
pub struct ProcessTable<T> {
  volatile: CachePadded<Volatile>,
  readonly: CachePadded<ReadOnly<T>>,
}

unsafe impl<T: Send> Send for ProcessTable<T> {}
unsafe impl<T: Send> Sync for ProcessTable<T> {}

impl<T> UnwindSafe for ProcessTable<T> {}
impl<T> RefUnwindSafe for ProcessTable<T> {}

impl<T> ProcessTable<T> {
  /// The minimum number of entries supported in the table.
  pub const MIN_ENTRIES: usize = 1 << 4;

  /// The maximum number of entries supported in the table.
  pub const MAX_ENTRIES: usize = {
    #[cfg(not(test))]
    {
      1 << 27
    }
    #[cfg(test)]
    {
      1 << 20
    }
  };

  /// The default number of entries supported in the table.
  pub const DEF_ENTRIES: usize = 1 << 20;

  /// Creates an empty `ProcessTable` with the default capacity.
  #[inline]
  pub fn new() -> Self {
    Self::with_capacity(Self::DEF_ENTRIES)
  }

  /// Creates an empty `ProcessTable` with at least the specified capacity.
  ///
  /// The table will be able to hold at least `capacity` entries and does not reallocate.
  pub fn with_capacity(capacity: usize) -> Self {
    // -------------------------------------------------------------------------
    // Determine Size
    // -------------------------------------------------------------------------

    let size: usize = Self::layout_capacity(capacity);
    let bits: u32 = bit_width(size.strict_sub(1));

    // -------------------------------------------------------------------------
    // Determine Layout
    // -------------------------------------------------------------------------

    let memory: NonZeroUsize = assert_nonzero(strict_align(size.strict_mul(ENTRY_SIZE)));
    let layout: Layout = Layout::from_size_align(memory.get(), CACHE_LINE).expect("valid layout");
    let blocks: NonZeroUsize = assert_nonzero(layout.size() / CACHE_LINE);

    assert!(blocks.is_power_of_two(), "block count must be a power of two");

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
    let ptr_data: NonNull<AtomicPtr<T>> = Self::alloc_data_array(layout, cap);
    let ptr_slot: NonNull<AtomicU64> = Self::alloc_slot_array(layout, blocks);

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
        ptr_data,
        ptr_slot,
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

  /// Returns the maximum number of entries that can fit in the table.
  #[inline]
  pub fn capacity(&self) -> usize {
    self.readonly.cap.get()
  }

  /// Returns the number of entries in the table.
  #[inline]
  pub fn len(&self) -> usize {
    self.volatile.load_len_relaxed() as usize
  }

  /// Returns `true` if the table contains no entries.
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

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

  /// Adds a new entry to the table.
  ///
  /// The value added is the result of the provided `init` function, which is
  /// called **after** space for the new entry has been reserved.
  ///
  /// # Init Function
  ///
  /// - The `init` function **must not fail**, as failure will leave the entry
  ///   slot permanently reserved and unusable.
  ///
  /// - Avoid large allocations or time-consuming operations inside `init`.
  ///
  /// - Do not call other `ProcessTable` functions from `init` to prevent
  ///   potential deadlocks.
  ///
  /// # Errors
  ///
  /// Returns [`ProcessTableFull`] if the table has reached maximum capacity.
  pub fn insert<F>(&self, init: F) -> Result<Arc<T>, ProcessTableFull>
  where
    F: FnOnce(&mut MaybeUninit<T>, RawPid),
  {
    let Some(permit) = self.reserve_slot() else {
      return Err(ProcessTableFull);
    };

    // Allocate space for the entry.
    let mut uninit: UniqueArc<MaybeUninit<T>> = UniqueArc::new_uninit();

    let base_index: u64 = self.acquire_slot(permit);
    let real_index: u64 = self.readonly.base_index_to_real_index(base_index);

    // Initialize the table entry.
    let data: Arc<T> = {
      let rpid: RawPid = self.readonly.base_index_to_pid(base_index);

      init(&mut uninit, rpid);

      // SAFETY: `uninit` was initialized by the provided `init` function.
      UniqueArc::shareable(unsafe { UniqueArc::assume_init(uninit) })
    };

    // Increment the refcount before storing the `Arc<T>` pointer.
    //
    // This is crucial to prevent an early drop since we're returning the owned
    // `Arc<T>` to the caller.
    let heap: *mut T = Arc::into_raw(Arc::clone(&data)).cast_mut();

    // Find our slot and store the entry.
    //
    // SAFETY: We only yield valid indices from `base_index_to_real_index`.
    let entry: &AtomicPtr<T> = unsafe { self.readonly.data_unchecked(real_index as usize) };

    entry.store(heap, Ordering::Relaxed);

    Ok(data)
  }

  /// Removes and returns the entry associated with the given `pid`,
  /// or `None` if the `pid` is not found in the table.
  ///
  /// Note: The provided `pid` *may* correspond to an older table entry, and
  /// this method will remove the newer entry if it occupies the same slot. This
  /// behavior is memory-safe and not considered `unsafe`, though it may be
  /// surprising to some users.
  pub fn remove(&self, pid: RawPid) -> Option<Arc<T>> {
    let index: u64 = self.readonly.pid_to_real_index(pid);

    // SAFETY: We only yield valid indices from `pid_to_real_index`.
    let entry: &AtomicPtr<T> = unsafe { self.readonly.data_unchecked(index as usize) };

    // Remove the current entry from the table, we'll dealloc at the end.
    let heap: *mut T = entry.swap(ptr::null_mut(), Ordering::Relaxed);

    // The slot was empty, this must be a stale PID.
    if heap.is_null() {
      return None;
    }

    // Calculate the next slot value.
    let data: u64 = self.refresh_data(pid);

    debug_assert!(data != RESERVED);
    debug_assert!(index == self.readonly.base_index_to_real_index(data));

    // Release the slot by storing a new slot value with an updated generation.
    self.release_slot(data);

    debug_assert!(self.len() > 0);

    let _ignore: u32 = self.volatile.decr_len_release();

    // SAFETY: The pointer was produced by `Arc::into_raw` in `Self::insert` and
    //         we immediately return the owned value to prevent any additional
    //         (mis)uses of the pointer.
    Some(unsafe { Arc::from_raw(heap) })
  }

  pub fn at(&self, index: usize) -> Option<Arc<T>> {
    let real: u64 = self.readonly.base_index_to_real_index(index as u64);
    let item: &AtomicPtr<T> = self.readonly.data(real as usize)?;
    let heap: *mut T = item.load(Ordering::Relaxed);

    if heap.is_null() {
      return None;
    }

    // TODO: This is not entirely safe and needs to be sorted out
    let heap: ManuallyDrop<Arc<T>> = ManuallyDrop::new(unsafe { Arc::from_raw(heap) });
    let data: Arc<T> = Arc::clone(&heap);

    Some(data)
  }

  #[inline]
  pub fn iter(&self) -> Iter<'_, T> {
    Iter::new(self)
  }

  // ---------------------------------------------------------------------------
  // Layout
  // ---------------------------------------------------------------------------

  const fn layout_capacity(capacity: usize) -> usize {
    if capacity < Self::MIN_ENTRIES {
      Self::MIN_ENTRIES
    } else if capacity > Self::MAX_ENTRIES {
      Self::MAX_ENTRIES
    } else {
      capacity.next_power_of_two()
    }
  }

  // ---------------------------------------------------------------------------
  // Block/Slot Allocation
  // ---------------------------------------------------------------------------

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

  fn reserve_slot(&self) -> Option<Permit<'_, T>> {
    // Optimistically reserve a slot.
    let count: u32 = self.volatile.incr_len_acquire();

    // Return a permit to the caller if the slot is valid.
    //
    // Note: The value of `count` is the previous value *before* it was
    //       incremented.
    if count < self.capacity() as u32 {
      return Some(Permit::new());
    }

    // If the slot is not valid, use a CAS loop to restore the proper count.
    //
    // Note: We increment *our* count because we are now comparing against the
    //       post-increment atomic value.
    let mut count: u32 = count + 1;

    'restore: loop {
      let Err(next) = self.volatile.swap_len_release(count, count - 1) else {
        break 'restore None;
      };

      // Another thread decremented the count which freed up a slot for us.
      if next <= self.capacity() as u32 {
        return Some(Permit::new());
      }

      // We failed the exchange so update our count and try again.
      count = next;
    }
  }

  fn acquire_slot(&self, _permit: Permit<'_, T>) -> u64 {
    'acquire: loop {
      let base_index: u64 = self.volatile.incr_aid_acquire();
      let real_index: u64 = self.readonly.base_index_to_real_index(base_index);

      // Find our slot and store the reservation marker.
      //
      // SAFETY: We only yield valid indices from `base_index_to_real_index`.
      let atomic: &AtomicU64 = unsafe { self.readonly.slot_unchecked(real_index as usize) };
      let result: u64 = atomic.swap(RESERVED, Ordering::Relaxed);

      // This slot was already reserved, try again.
      if result == RESERVED {
        continue 'acquire;
      }

      // Return the value stored in the slot (data array "base_index").
      break 'acquire result;
    }
  }

  fn release_slot(&self, update: u64) {
    'release: loop {
      // Increment the slot counter and convert to an index in the slot table.
      let base_index: u64 = self.volatile.incr_fid_release();
      let real_index: u64 = self.readonly.base_index_to_real_index(base_index);

      // Find our slot and store the updated value.
      //
      // SAFETY: We only yield valid indices from `base_index_to_real_index`.
      let atomic: &AtomicU64 = unsafe { self.readonly.slot_unchecked(real_index as usize) };

      let result: Result<u64, u64> =
        atomic.compare_exchange(RESERVED, update, Ordering::Release, Ordering::Relaxed);

      // This slot was not reserved, try again.
      if result.is_ok() {
        break 'release;
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Memory Allocation
  // ---------------------------------------------------------------------------

  #[inline]
  fn alloc_data_array(layout: Layout, count: NonZeroUsize) -> NonNull<AtomicPtr<T>> {
    // SAFETY: `alloc()` is safe to call here as the layout was validated in
    //         `Self::with_capacity`. We also initialize the array immediately
    //         afterwards, ensuring we only return valid data.
    let ptr: NonNull<AtomicPtr<T>> = unsafe { Self::alloc(layout) };

    // Initialize all data atomics with a NULL ptr.
    for offset in 0..count.get() {
      // SAFETY: `add` is safe as we are iterating over the allocated memory,
      //         and `write` is safe as we are initializing with a valid value.
      unsafe {
        ptr.add(offset).write(AtomicPtr::new(ptr::null_mut()));
      }
    }

    ptr
  }

  #[inline]
  fn alloc_slot_array(layout: Layout, blocks: NonZeroUsize) -> NonNull<AtomicU64> {
    // SAFETY: `alloc()` is safe to call here as the layout was validated in
    //         `Self::with_capacity`. We also initialize the array immediately
    //         afterwards, ensuring we only return valid data.
    let ptr: NonNull<AtomicU64> = unsafe { Self::alloc(layout) };

    let mut offset: usize = 0;

    // Initialize all slot atomics with a pre-computed cache-line slot.
    for block in 0..blocks.get() {
      for index in 0..SLOT_COUNT {
        let value: u64 = index
          .strict_mul(blocks.get())
          .strict_add(block)
          .try_into()
          .expect("valid default slot value");

        // SAFETY: `add` is safe as we are within the bounds of the allocation,
        //         and `write` is safe as we are initializing with a valid value.
        unsafe {
          ptr.add(offset).write(AtomicU64::new(value));
        }

        offset += 1;
      }
    }

    ptr
  }

  /// Deallocate the data array specified by `ptr`.
  ///
  /// # Safety
  ///
  /// `ptr` must have been previously allocated using `alloc()`,
  /// which ensures all invavriants are upheld.
  #[inline]
  unsafe fn dealloc_data_array(layout: Layout, ptr: NonNull<AtomicPtr<T>>) {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { Self::dealloc(layout, ptr) }
  }

  /// Deallocate the slot array specified by `ptr`.
  ///
  /// # Safety
  ///
  /// `ptr` must have been previously allocated using `alloc()`,
  /// which ensures all invavriants are upheld.
  #[inline]
  unsafe fn dealloc_slot_array(layout: Layout, ptr: NonNull<AtomicU64>) {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { Self::dealloc(layout, ptr) }
  }

  #[inline]
  unsafe fn alloc<A>(layout: Layout) -> NonNull<A> {
    // SAFETY: The layout was validated in `Self::with_capacity` to be non-zero
    //         and properly aligned.
    let Some(ptr) = NonNull::new(unsafe { alloc(layout) }) else {
      handle_alloc_error(layout);
    };

    ptr.cast()
  }

  #[inline]
  unsafe fn dealloc<A>(layout: Layout, ptr: NonNull<A>) {
    // SAFETY: The pointer is valid, as it's returned by the allocator. We are
    //         deallocating it using the correct layout for the memory.
    unsafe { dealloc(ptr.cast().as_ptr(), layout) }
  }
}

impl<T> Drop for ProcessTable<T> {
  fn drop(&mut self) {
    let memory: usize = strict_align(self.capacity().strict_mul(ENTRY_SIZE));
    let layout: Layout = Layout::from_size_align(memory, CACHE_LINE).expect("valid layout");

    // SAFETY: The `ptr_data` and `ptr_slot` pointers are guaranteed to be valid,
    //         as they are allocated using internal methods The `dealloc_*`
    //         methods are safe since we are passing the correct pointers.
    unsafe {
      Self::dealloc_data_array(layout, self.readonly.ptr_data);
      Self::dealloc_slot_array(layout, self.readonly.ptr_slot);
    }
  }
}

impl<T> Debug for ProcessTable<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.debug_struct("ProcessTable")
      .field("volatile.len", &self.volatile.len)
      .field("volatile.aid", &self.volatile.aid)
      .field("volatile.fid", &self.volatile.fid)
      .field("readonly.ptr_data", &self.readonly.ptr_data)
      .field("readonly.ptr_slot", &self.readonly.ptr_slot)
      .field("readonly.cap", &self.readonly.cap)
      .field("readonly.id_mask_entry", &format_args!("{:0>32b}", &self.readonly.id_mask_entry),)
      .field("readonly.id_mask_block", &format_args!("{:0>32b}", &self.readonly.id_mask_block),)
      .field("readonly.id_mask_index", &format_args!("{:0>32b}", &self.readonly.id_mask_index),)
      .field("readonly.id_shift_block", &self.readonly.id_shift_block)
      .field("readonly.id_shift_index", &self.readonly.id_shift_index)
      .finish()
  }
}

// -----------------------------------------------------------------------------
// Table State
// -----------------------------------------------------------------------------

#[repr(C)]
struct Volatile {
  /// The number of entries in the table.
  len: AtomicU32,
  /// The next available slot to allocate.
  aid: AtomicU64,
  /// The next available slot to free.
  fid: AtomicU64,
}

impl Volatile {
  /// Increments the `aid` counter with [`acquire`][Ordering::Acquire] ordering.
  fn incr_aid_acquire(&self) -> u64 {
    self.aid.fetch_add(1, Ordering::Acquire)
  }

  /// Increments the `fid` counter with [`release`][Ordering::Release] ordering.
  fn incr_fid_release(&self) -> u64 {
    self.fid.fetch_add(1, Ordering::Release)
  }

  /// Increments the `len` counter with [`acquire`][Ordering::Acquire] ordering.
  fn incr_len_acquire(&self) -> u32 {
    self.len.fetch_add(1, Ordering::Acquire)
  }

  /// Decrements the `len` counter with [`release`][Ordering::Release] ordering.
  fn decr_len_release(&self) -> u32 {
    self.len.fetch_sub(1, Ordering::Release)
  }

  /// Reads the `len` counter with [`relaxed`][Ordering::Relaxed] ordering.
  fn load_len_relaxed(&self) -> u32 {
    self.len.load(Ordering::Relaxed)
  }

  /// Stores `new` in the `len` counter if the current value matches `cmp`.
  ///
  /// Uses [`release`][Ordering::Release] for success and [`relaxed`][Ordering::Relaxed] for failure.
  fn swap_len_release(&self, cmp: u32, new: u32) -> Result<u32, u32> {
    self
      .len
      .compare_exchange(cmp, new, Ordering::Release, Ordering::Relaxed)
  }
}

#[repr(C)]
struct ReadOnly<T> {
  /// Pointer to the array of table entries.
  ptr_data: NonNull<AtomicPtr<T>>,
  /// Pointer to the array of slot locations.
  ptr_slot: NonNull<AtomicU64>,
  /// The maximum number of entries that can fit in the table.
  cap: NonZeroUsize,
  /// Id-formatting fields
  id_mask_entry: u32,
  id_mask_block: u32,
  id_mask_index: u32,
  id_shift_block: u32,
  id_shift_index: u32,
}

impl<T> ReadOnly<T> {
  /// Returns a reference to the data entry at `index`, or `None` if out of bounds.
  #[inline]
  fn data(&self, index: usize) -> Option<&AtomicPtr<T>> {
    if index < self.cap.get() {
      // SAFETY: `index` was just checked to be in bounds.
      Some(unsafe { self.data_unchecked(index) })
    } else {
      None
    }
  }

  /// Returns a reference to the slot entry at `index`, or `None` if out of bounds.
  #[inline]
  fn slot(&self, index: usize) -> Option<&AtomicU64> {
    if index < self.cap.get() {
      // SAFETY: `index` was just checked to be in bounds.
      Some(unsafe { self.slot_unchecked(index) })
    } else {
      None
    }
  }

  /// Returns a reference to the data entry at `index`, without doing safety checks.
  ///
  /// # Safety
  ///
  ///   - The `index` must be *less* than the value of [`cap`][Self::cap].
  ///   - The pointer must be [convertible to a reference].
  ///
  /// [convertible to a reference]: std::ptr#pointer-to-reference-conversion
  #[inline]
  const unsafe fn data_unchecked(&self, index: usize) -> &AtomicPtr<T> {
    #[cfg(debug_assertions)]
    if index >= self.cap.get() {
      panic!("ReadOnly::data_unchecked requires that the index is in bounds");
    }

    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.ptr_data.add(index).as_ref() }
  }

  /// Returns a reference to the slot entry at `index`, without doing safety checks.
  ///
  /// # Safety
  ///
  ///   - The `index` must be *less* than the value of [`cap`][Self::cap].
  ///   - The pointer must be [convertible to a reference].
  ///
  /// [convertible to a reference]: std::ptr#pointer-to-reference-conversion
  #[inline]
  const unsafe fn slot_unchecked(&self, index: usize) -> &AtomicU64 {
    #[cfg(debug_assertions)]
    if index >= self.cap.get() {
      panic!("ReadOnly::slot_unchecked requires that the index is in bounds");
    }

    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.ptr_slot.add(index).as_ref() }
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
  const fn pid_data_to_real_index(&self, input: u64) -> u64 {
    input & self.id_mask_entry as u64
  }

  #[inline]
  const fn base_index_to_real_index(&self, input: u64) -> u64 {
    let mut value: u64 = 0;
    value += (input & self.id_mask_block as u64) << self.id_shift_block;
    value += (input >> self.id_shift_index) & self.id_mask_index as u64;
    debug_assert!(value < self.cap.get() as u64);
    value
  }

  #[inline]
  const fn base_index_to_pid_data(&self, input: u64) -> u64 {
    let value: u64 = input & !(self.id_mask_entry as u64);
    let value: u64 = value | self.base_index_to_real_index(input);
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
  const fn pid_to_real_index(&self, input: RawPid) -> u64 {
    let value: u64 = input.into_bits() >> RawPid::TAG_BITS;
    let value: u64 = self.pid_data_to_real_index(value);
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

  type Table = ProcessTable<u8>;

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
  fn test_layout_capacity() {
    let min: usize = Table::MIN_ENTRIES;
    let max: usize = Table::MAX_ENTRIES;

    for capacity in (min..=max) {
      assert_eq!(
        Table::layout_capacity(capacity),
        capacity.next_power_of_two()
      );
    }
  }

  #[test]
  fn test_id_mapping() {
    let table: Table = Table::with_capacity(1 << 5);

    for index in 0..table.capacity() * 3 {
      let real: u64 = table.readonly.base_index_to_real_index(index as u64);
      let data: u64 = table.readonly.base_index_to_pid_data(index as u64);
      let rpid: RawPid = table.readonly.base_index_to_pid(index as u64);

      assert_eq!(table.readonly.pid_data_to_base_index(data), index as u64);
      assert_eq!(table.readonly.pid_data_to_real_index(data), real);

      assert_eq!(table.readonly.pid_to_real_index(rpid), real);
      assert_eq!(table.readonly.pid_to_base_index(rpid), index as u64);
    }
  }

  #[test]
  fn test_len() {
    let table: Table = Table::with_capacity(1 << 5);

    assert_eq!(table.len(), 0);

    table.insert(|entry, _| {
      entry.write(0_u8);
    });

    assert_eq!(table.len(), 1);

    table.insert(|entry, _| {
      entry.write(0_u8);
    });

    assert_eq!(table.len(), 2);
  }
}
