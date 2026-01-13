use hashbrown::HashMap;
use parking_lot::RwLock;
use parking_lot::RwLockReadGuard;
use parking_lot::RwLockUpgradableReadGuard;
use parking_lot::RwLockWriteGuard;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::slice::SliceIndex;

use crate::erts::Runtime;

#[derive(Debug)]
pub(crate) enum AtomTableError {
  AtomTooLarge,
  TooManyAtoms,
}

#[repr(transparent)]
pub(crate) struct AtomTable {
  inner: RwLock<Table>,
}

impl AtomTable {
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      inner: RwLock::new(Table::new()),
    }
  }

  pub(crate) fn get(&self, slot: u32) -> Option<&str> {
    let guard: RwLockReadGuard<'_, Table> = self.inner.read();
    let index: usize = slot as usize;

    if index >= guard.len {
      return None;
    }

    // SAFETY: We just ensured the `index` is in bounds.
    let block: &Block = unsafe { guard.arr.get_unchecked(index >> Block::BITS) };
    let value: &'static str = unsafe { block.get_unchecked(index & Block::MASK) };

    Some(value)
  }

  pub(crate) fn set(&self, data: &str) -> Result<u32, AtomTableError> {
    // -------------------------------------------------------------------------
    // 1. Fast Path - Existing Atom
    // -------------------------------------------------------------------------

    let guard: RwLockUpgradableReadGuard<'_, Table> = self.inner.upgradable_read();

    if let Some(index) = guard.map.get(data).copied() {
      return Ok(index);
    }

    // -------------------------------------------------------------------------
    // 2. Slow Path - New Atom
    // -------------------------------------------------------------------------

    let mut guard: RwLockWriteGuard<'_, Table> = RwLockUpgradableReadGuard::upgrade(guard);

    if data.chars().count() > Runtime::MAX_ATOM_CHARS {
      return Err(AtomTableError::AtomTooLarge);
    }

    let cap: usize = guard.cap();
    let len: usize = guard.len();

    if len >= cap {
      if len >= Runtime::MAX_ATOM_COUNT {
        return Err(AtomTableError::TooManyAtoms);
      }

      guard.arr.push(Block::new());
    }

    debug_assert!(guard.arr.len() == (len >> Block::BITS) + 1);
    debug_assert!(Block::SIZE > (len & Block::MASK));

    // SAFETY: The total number of blocks is determined by `Runtime::MAX_ATOM_COUNT`
    //         and this is checked above to ensure we never hand out an index beyond
    //         our maximum capacity. We also split the index in two parts:
    //         - The top bits determine the block index (ie. `index >> Block::BITS`)
    //         - The bottom bits determine the local block slot (`len & Block::MASK`)
    //         These simple invariants ensure we are accessing an in-bounds index.
    let block: &mut Block = unsafe { guard.arr.get_unchecked_mut(len >> Block::BITS) };
    let slot: &mut &'static str = unsafe { block.get_unchecked_mut(len & Block::MASK) };
    let term: &'static str = Box::leak(Box::from(data));

    *slot = term;
    guard.map.insert(term, len as u32);
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
  map: HashMap<&'static str, u32>,
  arr: Vec<Block>,
  len: usize,
}

impl Table {
  const BLOCKS: usize = Runtime::MAX_ATOM_COUNT
    .strict_add(Block::SIZE)
    .strict_sub(1)
    .strict_div(Block::SIZE);

  #[inline]
  fn new() -> Self {
    Self {
      map: HashMap::with_capacity(Block::SIZE),
      arr: Vec::with_capacity(Self::BLOCKS),
      len: 0,
    }
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
