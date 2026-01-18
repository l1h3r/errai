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

#[derive(Debug)]
#[non_exhaustive]
pub enum AtomTableError {
  AtomTooLarge,
  TooManyAtoms,
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

/// Atom interning table.
///
/// # Memory behavior
///
/// Atoms are interned permanently and are **never deallocated** for the
/// lifetime of the runtime.
///
/// To mitigate unbounded growth, the table enforces the following limits:
///
/// - `MAX_ATOM_COUNT`: maximum number of distinct atoms
/// - `MAX_ATOM_BYTES`: maximum UTF-8 byte length per atom
///
/// These limits provide operational safety but do **not** make atom creation
/// reversible. Creating atoms dynamically from untrusted input may still lead
/// to memory exhaustion and should be avoided.
#[repr(transparent)]
pub struct AtomTable {
  inner: RwLock<Table>,
}

impl AtomTable {
  #[inline]
  pub fn new() -> Self {
    Self {
      inner: RwLock::new(Table::new()),
    }
  }

  pub fn get(&self, slot: u32) -> Result<&str, AtomTableError> {
    let guard: RwLockReadGuard<'_, Table> = self.inner.read();
    let index: usize = slot as usize;

    if index >= guard.len {
      return Err(AtomTableError::AtomNotFound);
    }

    // SAFETY: We just ensured the `index` is in bounds.
    let block: &Block = unsafe { guard.arr.get_unchecked(index >> Block::BITS) };
    let value: &'static str = unsafe { block.get_unchecked(index & Block::MASK) };

    Ok(value)
  }

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
    let block: &mut Block = unsafe { guard.arr.get_unchecked_mut(len >> Block::BITS) };
    let slot: &mut &'static str = unsafe { block.get_unchecked_mut(len & Block::MASK) };
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

#[repr(C)]
struct Table {
  map: HashMap<&'static str, u32, DefaultHashBuilder>,
  arr: Vec<Block>,
  len: usize,
}

impl Table {
  const BLOCKS: usize = MAX_ATOM_COUNT
    .strict_add(Block::SIZE)
    .strict_sub(1)
    .strict_div(Block::SIZE);

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

  #[inline]
  fn cap(&self) -> usize {
    self.arr.len() * Block::SIZE
  }

  #[inline]
  fn len(&self) -> usize {
    self.len
  }
}

// -----------------------------------------------------------------------------
// Atom Table - Block
// -----------------------------------------------------------------------------

#[repr(transparent)]
struct Block {
  inner: [&'static str; Self::SIZE],
}

impl Block {
  const BITS: u32 = 10;
  const SIZE: usize = 1_usize.strict_shl(Self::BITS);
  const MASK: usize = Self::SIZE.strict_sub(1);

  #[inline]
  const fn new() -> Self {
    Self {
      inner: [""; Self::SIZE],
    }
  }

  #[inline]
  unsafe fn get_unchecked<I>(&self, index: I) -> &<I as SliceIndex<[&'static str]>>::Output
  where
    I: SliceIndex<[&'static str]>,
  {
    // SAFETY: This is guaranteed to be safe by the caller.
    unsafe { self.inner.get_unchecked(index) }
  }

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
