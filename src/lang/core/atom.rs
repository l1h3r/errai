use hashbrown::HashMap;
use parking_lot::RwLock;
use parking_lot::RwLockReadGuard;
use parking_lot::RwLockUpgradableReadGuard;
use parking_lot::RwLockWriteGuard;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;
use std::slice::SliceIndex;
use std::sync::LazyLock;

use crate::core::raise;
use crate::erts::Runtime;

/// A static literal term.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
#[repr(transparent)]
pub struct Atom {
  slot: u32,
}

impl Atom {
  pub const FALSE: Self = Self::from_slot(1);
  pub const TRUE: Self = Self::from_slot(2);
  pub const NONODE_NOHOST: Self = Self::from_slot(3);
  pub const INFINITY: Self = Self::from_slot(4);
  pub const TIMEOUT: Self = Self::from_slot(5);
  pub const NORMAL: Self = Self::from_slot(6);
  pub const CALL: Self = Self::from_slot(7);
  pub const RETURN: Self = Self::from_slot(8);
  pub const THROW: Self = Self::from_slot(9);
  pub const ERROR: Self = Self::from_slot(10);
  pub const EXIT: Self = Self::from_slot(11);
  pub const UNDEFINED: Self = Self::from_slot(12);

  /// Creates a new system `Atom`.
  #[inline]
  const fn from_slot(slot: u32) -> Self {
    Self { slot }
  }

  /// Creates a new `Atom` from the given `data`.
  #[inline]
  pub fn new(data: &str) -> Self {
    Self::from_slot(TABLE.set(data))
  }

  #[inline]
  pub fn as_str(&self) -> &str {
    let Some(term) = TABLE.get(self.slot) else {
      raise!(Error, SysInv, "atom not found");
    };

    term
  }
}

impl Debug for Atom {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str(self.as_str())
  }
}

impl Display for Atom {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    f.write_str(self.as_str())
  }
}

impl PartialOrd for Atom {
  #[inline]
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for Atom {
  fn cmp(&self, other: &Self) -> Ordering {
    self.as_str().cmp(other.as_str())
  }
}

impl<'a> From<&'a str> for Atom {
  #[inline]
  fn from(other: &'a str) -> Self {
    Self::new(other)
  }
}

impl From<String> for Atom {
  #[inline]
  fn from(other: String) -> Self {
    Self::new(other.as_str())
  }
}

impl<'a> From<&'a String> for Atom {
  #[inline]
  fn from(other: &'a String) -> Self {
    Self::new(other.as_str())
  }
}

// -----------------------------------------------------------------------------
// Atom Table
// -----------------------------------------------------------------------------

static TABLE: LazyLock<AtomTable> = LazyLock::new(|| {
  let table: AtomTable = AtomTable::new();

  assert_eq!(table.set(""), 0);
  assert_eq!(table.set("false"), 1);
  assert_eq!(table.set("true"), 2);
  assert_eq!(table.set("nonode@nohost"), 3);

  assert_eq!(table.set("infinity"), 4);
  assert_eq!(table.set("timeout"), 5);
  assert_eq!(table.set("normal"), 6);
  assert_eq!(table.set("call"), 7);
  assert_eq!(table.set("return"), 8);

  assert_eq!(table.set("throw"), 9);
  assert_eq!(table.set("error"), 10);
  assert_eq!(table.set("exit"), 11);

  assert_eq!(table.set("undefined"), 12);

  table
});

#[repr(transparent)]
struct AtomTable {
  inner: RwLock<Table>,
}

impl AtomTable {
  /// Creates a new, empty `AtomTable`.
  #[inline]
  fn new() -> Self {
    Self {
      inner: RwLock::new(Table::new()),
    }
  }

  fn get(&self, slot: u32) -> Option<&str> {
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

  fn set(&self, data: &str) -> u32 {
    // -------------------------------------------------------------------------
    // 1. Fast Path - Existing Atom
    // -------------------------------------------------------------------------

    let guard: RwLockUpgradableReadGuard<'_, Table> = self.inner.upgradable_read();

    if let Some(index) = guard.map.get(data).copied() {
      return index;
    }

    // -------------------------------------------------------------------------
    // 2. Slow Path - New Atom
    // -------------------------------------------------------------------------

    let mut guard: RwLockWriteGuard<'_, Table> = RwLockUpgradableReadGuard::upgrade(guard);

    if data.chars().count() > Runtime::MAX_ATOM_CHARS {
      raise!(Error, SysCap, "atom too large");
    }

    let cap: usize = guard.cap();
    let len: usize = guard.len();

    if len >= cap {
      if len >= Runtime::MAX_ATOM_COUNT {
        raise!(Error, SysCap, "too many atoms");
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

    len as u32
  }
}

impl Debug for AtomTable {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    Debug::fmt(&self.inner, f)
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

impl Debug for Table {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    let inverted: BTreeMap<u32, &'static str> =
      self.map.iter().map(|(key, value)| (*value, *key)).collect();

    f.debug_struct("Table")
      .field("data", &inverted)
      .field("size", &self.len)
      .finish()
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
