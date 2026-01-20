//! Global atom interning table with permanent storage semantics.
//!
//! This module provides a thread-safe string interning table that permanently
//! stores unique string values. Once interned, atoms are never deallocated and
//! can be referenced by their numeric slot identifier.
//!
//! # Atom Semantics
//!
//! Atoms are immutable, interned strings with the following properties:
//!
//! - **Permanent storage**: Once created, atoms live for the runtime's lifetime
//! - **Unique representation**: Each distinct string is stored exactly once
//! - **Fast comparison**: Atoms can be compared by their slot index (u32)
//! - **Bounded capacity**: Limited to [`MAX_ATOM_COUNT`] distinct atoms
//!
//! # Thread Safety
//!
//! The atom table uses a read-write lock with an optimized fast path for
//! existing atoms. Most lookups only require a read lock, while new atom
//! creation requires a write lock.
//!
//! # Memory Considerations
//!
//! Atoms are **never deallocated**. Each interned string consumes memory
//! permanently. To prevent unbounded growth:
//!
//! - Maximum atom count: [`MAX_ATOM_COUNT`]
//! - Maximum atom size: [`MAX_ATOM_BYTES`]
//!
//! Avoid creating atoms from untrusted or dynamic input that could exhaust
//! the table capacity.

use hashbrown::DefaultHashBuilder;
use hashbrown::HashMap;
use parking_lot::RwLock;
use parking_lot::RwLockReadGuard;
use parking_lot::RwLockUpgradableReadGuard;
use parking_lot::RwLockWriteGuard;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::hash::BuildHasher;
use std::slice::SliceIndex;

use crate::consts::MAX_ATOM_BYTES;
use crate::consts::MAX_ATOM_COUNT;

// -----------------------------------------------------------------------------
// Atom Table Error
// -----------------------------------------------------------------------------

/// Errors returned from atom table lookup or insertion operations.
///
/// These errors indicate capacity limits or invalid atom access.
#[derive(Debug)]
#[non_exhaustive]
pub enum AtomTableError {
  /// The atom exceeds the maximum allowed byte length.
  ///
  /// Atoms are limited to [`MAX_ATOM_BYTES`] UTF-8 bytes to prevent
  /// excessive memory consumption per atom.
  AtomTooLarge,
  /// The atom table has reached its maximum capacity.
  ///
  /// The table is limited to [`MAX_ATOM_COUNT`] distinct atoms. This
  /// prevents unbounded memory growth from dynamic atom creation.
  TooManyAtoms,
  /// The requested atom slot does not exist.
  ///
  /// This indicates an invalid slot index was provided to [`AtomTable::get()`].
  AtomNotFound,
}

impl Display for AtomTableError {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    match self {
      Self::AtomTooLarge => f.write_str("atom too large"),
      Self::TooManyAtoms => f.write_str("too many atoms"),
      Self::AtomNotFound => f.write_str("atom not found"),
    }
  }
}

impl Error for AtomTableError {}

// -----------------------------------------------------------------------------
// Atom Table
// -----------------------------------------------------------------------------

/// Thread-safe atom interning table with permanent storage.
///
/// This table stores unique strings permanently and provides fast lookups
/// via numeric slot identifiers. Interned strings are never deallocated.
///
/// # Implementation Details
///
/// The table uses a two-level structure:
///
/// 1. **HashMap**: Maps strings to slot indices for O(1) lookup
/// 2. **Block array**: Stores the actual string data in fixed-size blocks
///
/// This design balances memory efficiency with lookup performance while
/// supporting lock-free reads for existing atoms.
///
/// # Memory Behavior
///
/// Atoms are interned permanently and are **never deallocated** for the
/// lifetime of the runtime.
///
/// To mitigate unbounded growth, the table enforces the following limits:
///
/// - [`MAX_ATOM_COUNT`]: maximum number of distinct atoms
/// - [`MAX_ATOM_BYTES`]: maximum UTF-8 byte length per atom
///
/// These limits provide operational safety but do **not** make atom creation
/// reversible. Creating atoms dynamically from untrusted input may still lead
/// to memory exhaustion and should be avoided.
#[repr(transparent)]
pub struct AtomTable {
  inner: RwLock<Table>,
}

impl AtomTable {
  /// Creates a new empty atom table with initial capacity allocated.
  ///
  /// The table is pre-allocated with space for the first block of atoms
  /// to avoid early allocations during startup.
  #[inline]
  pub fn new() -> Self {
    Self {
      inner: RwLock::new(Table::new()),
    }
  }

  /// Returns the atom string for the given table slot.
  ///
  /// This operation only requires a read lock and is highly concurrent.
  ///
  /// # Errors
  ///
  /// Returns [`AtomTableError::AtomNotFound`] if the slot is invalid or
  /// has not been allocated yet.
  pub fn get(&self, slot: u32) -> Result<&str, AtomTableError> {
    let guard: RwLockReadGuard<'_, Table> = self.inner.read();
    let index: usize = slot as usize;

    if index >= guard.len {
      return Err(AtomTableError::AtomNotFound);
    }

    // SAFETY: We just ensured the `index` is in bounds.
    let value: &'static str = unsafe {
      guard
        .arr
        .get_unchecked(index >> Block::BITS)
        .get_unchecked(index & Block::MASK)
    };

    Ok(value)
  }

  /// Interns a string and returns its atom table slot.
  ///
  /// If the string is already interned, returns the existing slot without
  /// modification. Otherwise, allocates a new slot and stores the string.
  ///
  /// # Concurrency
  ///
  /// This method uses an optimized two-phase locking strategy:
  ///
  /// 1. **Fast path**: Acquires read lock to check for existing atom
  /// 2. **Slow path**: Upgrades to write lock only for new atoms
  ///
  /// Most calls only require a read lock, providing excellent concurrency
  /// for workloads with repeated atom creation.
  ///
  /// # Errors
  ///
  /// Returns [`AtomTableError::AtomTooLarge`] if the string exceeds
  /// [`MAX_ATOM_BYTES`].
  ///
  /// Returns [`AtomTableError::TooManyAtoms`] if the table has reached
  /// [`MAX_ATOM_COUNT`] capacity.
  pub fn set(&self, data: &str) -> Result<u32, AtomTableError> {
    // -------------------------------------------------------------------------
    // 1. Fast Path - Existing Atom
    // -------------------------------------------------------------------------

    let guard: RwLockUpgradableReadGuard<'_, Table> = self.inner.upgradable_read();
    let dhash: u64 = guard.map.hasher().hash_one(data);

    if let Some((_, index)) = guard.map.raw_entry().from_key_hashed_nocheck(dhash, data) {
      return Ok(*index);
    }

    // -------------------------------------------------------------------------
    // 2. Slow Path - New Atom
    // -------------------------------------------------------------------------

    let mut guard: RwLockWriteGuard<'_, Table> = RwLockUpgradableReadGuard::upgrade(guard);

    if data.len() > MAX_ATOM_BYTES {
      return Err(AtomTableError::AtomTooLarge);
    }

    let cap: usize = guard.cap();
    let len: usize = guard.len();

    if len >= cap {
      if len >= MAX_ATOM_COUNT {
        return Err(AtomTableError::TooManyAtoms);
      }

      guard.arr.push(Block::new());
    }

    debug_assert!(guard.arr.len() == (len >> Block::BITS) + 1);
    debug_assert!(Block::SIZE > (len & Block::MASK));

    // SAFETY:
    //
    // - `len` is the total number of interned atoms and is always <= MAX_ATOM_COUNT.
    // - Blocks are append-only and sized to cover all indices < `len`.
    // - The block index (`len >> Block::BITS`) and slot index (`len & Block::MASK`)
    //   therefore always refer to a valid, in-bounds location.
    let slot: &mut &'static str = unsafe {
      guard
        .arr
        .get_unchecked_mut(len >> Block::BITS)
        .get_unchecked_mut(len & Block::MASK)
    };

    let term: &'static str = Box::leak(Box::from(data));

    *slot = term;

    guard
      .map
      .raw_entry_mut()
      .from_key_hashed_nocheck(dhash, data)
      .insert(term, len as u32);

    guard.len += 1;

    drop(guard);

    Ok(len as u32)
  }
}

impl Debug for AtomTable {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    let guard: RwLockReadGuard<'_, Table> = self.inner.read();
    let items: _ = guard.map.iter().map(|(key, value)| (*value, *key));

    f.debug_struct("AtomTable")
      .field("data", &BTreeMap::from_iter(items))
      .field("size", &guard.len)
      .finish()
  }
}

// -----------------------------------------------------------------------------
// Atom Table - Table
// -----------------------------------------------------------------------------

/// Internal table structure holding atom data and lookup map.
///
/// This structure is protected by the [`AtomTable`]'s [`RwLock`] and should
/// not be accessed directly.
#[repr(C)]
struct Table {
  /// Maps interned strings to their slot indices for fast lookup.
  map: HashMap<&'static str, u32, DefaultHashBuilder>,
  /// Array of blocks storing the actual string data.
  arr: Vec<Block>,
  /// Total number of interned atoms across all blocks.
  len: usize,
}

impl Table {
  /// Maximum number of blocks needed to store [`MAX_ATOM_COUNT`] atoms.
  const BLOCKS: usize = MAX_ATOM_COUNT
    .strict_add(Block::SIZE)
    .strict_sub(1)
    .strict_div(Block::SIZE);

  /// Creates a new table with one pre-allocated block.
  #[inline]
  fn new() -> Self {
    let mut this: Self = Self {
      map: HashMap::with_capacity(Block::SIZE),
      arr: Vec::with_capacity(Self::BLOCKS),
      len: 0,
    };

    this.arr.push(Block::new());
    this
  }

  /// Returns the current total capacity across all allocated blocks.
  #[inline]
  fn cap(&self) -> usize {
    self.arr.len() * Block::SIZE
  }

  /// Returns the number of interned atoms.
  #[inline]
  fn len(&self) -> usize {
    self.len
  }
}

// -----------------------------------------------------------------------------
// Atom Table - Block
// -----------------------------------------------------------------------------

/// Fixed-size storage block for atom strings.
///
/// Blocks organize atoms into fixed-size arrays to enable efficient indexing
/// via bit manipulation. Each block stores [`Block::SIZE`] atom slots.
#[repr(transparent)]
struct Block {
  inner: [&'static str; Self::SIZE],
}

impl Block {
  /// Bit width for block-local slot indexing.
  const BITS: u32 = 10;

  /// Number of atom slots per block (2^10 = 1024).
  const SIZE: usize = 1_usize.strict_shl(Self::BITS);

  /// Bitmask for extracting block-local slot index.
  const MASK: usize = Self::SIZE.strict_sub(1);

  /// Creates a new block with all slots initialized to empty strings.
  #[inline]
  const fn new() -> Self {
    Self {
      inner: [""; Self::SIZE],
    }
  }

  /// Returns a reference to the atom at the given block-local index.
  ///
  /// # Safety
  ///
  /// The caller must ensure `index` is within bounds.
  #[inline]
  unsafe fn get_unchecked<I>(&self, index: I) -> &<I as SliceIndex<[&'static str]>>::Output
  where
    I: SliceIndex<[&'static str]>,
  {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.inner.get_unchecked(index) }
  }

  /// Returns a mutable reference to the atom at the given block-local index.
  ///
  /// # Safety
  ///
  /// The caller must ensure `index` is within bounds.
  #[inline]
  unsafe fn get_unchecked_mut<I>(
    &mut self,
    index: I,
  ) -> &mut <I as SliceIndex<[&'static str]>>::Output
  where
    I: SliceIndex<[&'static str]>,
  {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.inner.get_unchecked_mut(index) }
  }
}

#[cfg(test)]
mod tests {
  use std::thread;
  use triomphe::Arc;

  use crate::core::table::AtomTable;
  use crate::loom::sync::Barrier;

  #[test]
  fn stress_concurrent_same_atom() {
    let table: Arc<AtomTable> = Arc::new(AtomTable::new());
    let barrier: Arc<Barrier> = Arc::new(Barrier::new(100));

    let threads: Vec<_> = (0..100)
      .map(|_| {
        let table: Arc<AtomTable> = Arc::clone(&table);
        let barrier: Arc<Barrier> = Arc::clone(&barrier);

        thread::spawn(move || {
          barrier.wait();
          table.set("test").unwrap()
        })
      })
      .collect();

    let indices: Vec<u32> = threads
      .into_iter()
      .map(|handle| handle.join().unwrap())
      .collect();

    assert!(indices.windows(2).all(|window| window[0] == window[1]));
  }
}
