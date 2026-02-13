use ptab::DefaultParams;
use ptab::Detached;
use ptab::PTab;
use ptab::WeakKeys;
use std::error::Error;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use std::mem::MaybeUninit;

use crate::core::LocalPid;

// -----------------------------------------------------------------------------
// ProcAccessError
// -----------------------------------------------------------------------------

/// Error returned when attempting to access an invalid process.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) struct ProcAccessError;

impl Display for ProcAccessError {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    f.write_str("invalid process access")
  }
}

impl Error for ProcAccessError {}

// -----------------------------------------------------------------------------
// ProcInsertError
// -----------------------------------------------------------------------------

/// Error returned when the process table has reached its capacity.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) struct ProcInsertError;

impl Display for ProcInsertError {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    f.write_str("too many processes")
  }
}

impl Error for ProcInsertError {}

// -----------------------------------------------------------------------------
// Process Table
// -----------------------------------------------------------------------------

#[repr(transparent)]
pub(crate) struct ProcTable<T> {
  inner: PTab<T, DefaultParams>,
}

impl<T> ProcTable<T> {
  #[inline]
  pub(crate) fn new() -> Self {
    Self {
      inner: PTab::new(),
    }
  }

  #[inline]
  pub(crate) const fn capacity(&self) -> usize {
    self.inner.capacity()
  }

  #[inline]
  pub(crate) fn len(&self) -> usize {
    self.inner.len()
  }

  #[inline]
  pub(crate) fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  #[inline]
  pub(crate) fn insert(&self, value: T) -> Option<LocalPid>
  where
    T: 'static,
  {
    self.inner.insert(value).map(Into::into)
  }

  #[inline]
  pub(crate) fn write<F>(&self, init: F) -> Option<LocalPid>
  where
    T: 'static,
    F: FnOnce(&mut MaybeUninit<T>, LocalPid),
  {
    self
      .inner
      .write(|value, index| init(value, index.into()))
      .map(Into::into)
  }

  #[inline]
  pub(crate) fn remove(&self, index: LocalPid) -> bool {
    self.inner.remove(index.into())
  }

  #[inline]
  pub(crate) fn exists(&self, index: LocalPid) -> bool {
    self.inner.exists(index.into())
  }

  #[inline]
  pub(crate) fn with<F, R>(&self, index: LocalPid, f: F) -> Option<R>
  where
    F: Fn(&T) -> R,
  {
    self.inner.with(index.into(), f)
  }

  #[inline]
  pub(crate) fn read(&self, index: LocalPid) -> Option<T>
  where
    T: Copy,
  {
    self.inner.read(index.into())
  }

  #[inline]
  pub(crate) fn weak_keys(&self) -> WeakKeys<'_, T, DefaultParams> {
    self.inner.weak_keys()
  }
}

impl<T> Debug for ProcTable<T> {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    f.write_str("ProcTable { .. }")
  }
}

// -----------------------------------------------------------------------------
// LocalPid <-> Detached
// -----------------------------------------------------------------------------

impl From<Detached> for LocalPid {
  #[inline]
  fn from(other: Detached) -> Self {
    debug_assert!(other.into_bits() & Self::PID_MASK == other.into_bits());

    let value: usize = other.into_bits() & Self::PID_MASK;
    let value: usize = (value << Self::TAG_BITS) | Self::TAG_DATA;

    Self::from_bits(value)
  }
}

impl From<LocalPid> for Detached {
  #[inline]
  fn from(other: LocalPid) -> Self {
    Detached::from_bits(other.into_bits() >> LocalPid::TAG_BITS)
  }
}
