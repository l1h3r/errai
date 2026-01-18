use hashbrown::HashMap;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result;
use std::hash::Hash;

use crate::consts::CAP_PROC_DICTIONARY;
use crate::core::Atom;
use crate::core::Term;

/// Process dictionary providing per-process key-value storage.
///
/// The dictionary is lazily allocated on first write to avoid overhead
/// for processes that never use it. Most processes don't use the dictionary,
/// making this a worthwhile optimization.
///
/// # Lazy Allocation
///
/// The underlying [`HashMap`] is `None` until the first `insert()` call.
/// Read operations on an empty dictionary return `None` without allocation.
#[repr(transparent)]
pub(crate) struct ProcDict {
  inner: Option<HashMap<Atom, Term>>,
}

impl ProcDict {
  /// Allocates the underlying hash map with initial capacity.
  #[inline]
  fn alloc_table() -> HashMap<Atom, Term> {
    HashMap::with_capacity(CAP_PROC_DICTIONARY)
  }

  /// Creates a new, empty process dictionary.
  ///
  /// No allocation occurs until the first insertion.
  #[inline]
  pub(crate) fn new() -> Self {
    Self { inner: None }
  }

  /// Returns a cloned copy of the value for `key`.
  ///
  /// Returns `None` if the key is not present or the dictionary is empty.
  pub(crate) fn get<Q>(&self, key: &Q) -> Option<Term>
  where
    Atom: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    self.inner.as_ref().and_then(|data| data.get(key).cloned())
  }

  /// Inserts a key-value pair into the dictionary.
  ///
  /// Allocates the underlying map on first insertion. Returns the previous
  /// value if the key was already present.
  pub(crate) fn insert(&mut self, atom: Atom, term: Term) -> Option<Term> {
    self
      .inner
      .get_or_insert_with(Self::alloc_table)
      .insert(atom, term)
  }

  /// Removes `key` from the dictionary and returns its value.
  ///
  /// Returns `None` if the key is not present or the dictionary is empty.
  pub(crate) fn remove<Q>(&mut self, key: &Q) -> Option<Term>
  where
    Atom: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    self.inner.as_mut().and_then(|data| data.remove(key))
  }

  /// Clears the dictionary and returns all key-value pairs.
  ///
  /// Returns an empty vector if the dictionary was never allocated.
  #[inline]
  pub(crate) fn clear(&mut self) -> Vec<(Atom, Term)> {
    self
      .inner
      .as_mut()
      .map(|data| Vec::from_iter(data.drain()))
      .unwrap_or_default()
  }

  /// Returns a list of all key-value pairs in the dictionary.
  ///
  /// Returns an empty vector if the dictionary is empty or unallocated.
  pub(crate) fn pairs(&self) -> Vec<(Atom, Term)> {
    #[inline]
    fn clone((atom, term): (&Atom, &Term)) -> (Atom, Term) {
      (*atom, term.clone())
    }

    self
      .inner
      .as_ref()
      .map(|data| Vec::from_iter(data.iter().map(clone)))
      .unwrap_or_default()
  }

  /// Returns a list of all keys in the dictionary.
  ///
  /// Returns an empty vector if the dictionary is empty or unallocated.
  pub(crate) fn keys(&self) -> Vec<Atom> {
    self
      .inner
      .as_ref()
      .map(|data| Vec::from_iter(data.keys().copied()))
      .unwrap_or_default()
  }

  /// Returns a list of all values in the dictionary.
  ///
  /// Returns an empty vector if the dictionary is empty or unallocated.
  pub(crate) fn values(&self) -> Vec<Term> {
    self
      .inner
      .as_ref()
      .map(|data| Vec::from_iter(data.values().cloned()))
      .unwrap_or_default()
  }
}

impl Debug for ProcDict {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    if let Some(data) = self.inner.as_ref() {
      f.write_str("ProcDict ")?;
      f.debug_map().entries(data.iter()).finish()
    } else {
      f.write_str("ProcDict {}")
    }
  }
}
