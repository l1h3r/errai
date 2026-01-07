use hashbrown::HashMap;
use parking_lot::RwLock;
use std::borrow::Borrow;
use std::hash::Hash;

use crate::lang::Atom;
use crate::lang::Term;

/// A process dictionary.
#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct ProcessDict {
  inner: RwLock<Option<HashMap<Atom, Term>>>,
}

unsafe impl Send for ProcessDict {}
unsafe impl Sync for ProcessDict {}

impl ProcessDict {
  /// The number of pre-allocated entries in the process dictionary.
  const CAP_DICTIONARY: usize = 8;

  #[inline]
  fn alloc_table() -> HashMap<Atom, Term> {
    HashMap::with_capacity(Self::CAP_DICTIONARY)
  }

  /// Creates a new `ProcessDict`.
  #[inline]
  pub(crate) const fn new() -> Self {
    Self {
      inner: RwLock::new(None),
    }
  }

  /// Returns a cloned copy of the term corresponding to `key`.
  pub(crate) fn get<Q>(&self, key: &Q) -> Option<Term>
  where
    Atom: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    // Acquire read access to the inner table - this always succeeds
    // as our borrows are fully contained within the public API.
    let Some(guard) = self.inner.try_read() else {
      unreachable!()
    };

    // Retrieve an immutable reference to the underlying table;
    // bail early if the table was not initialized.
    let Some(data) = guard.as_ref() else {
      return None;
    };

    // Copy the key-value pair from the table.
    data.get(key).cloned()
  }

  /// Inserts a key-value pair into the table.
  ///
  /// Returns the previous value, or `None`.
  pub(crate) fn insert(&self, atom: Atom, term: Term) -> Option<Term> {
    // Acquire write access to the inner table - this always succeeds
    // as our borrows are fully contained within the public API.
    let Some(mut guard) = self.inner.try_write() else {
      unreachable!()
    };

    // Retrieve a mutable reference to the underlying table;
    // also allocate a default table if necessary.
    let data: &mut HashMap<Atom, Term> = guard.get_or_insert_with(Self::alloc_table);

    // Add the new key-value pair to the table and return any previous value.
    data.insert(atom, term)
  }

  /// Removes `key` from the table and returns the value if it was set,
  /// otherwise returns `None`.
  pub(crate) fn remove<Q>(&self, key: &Q) -> Option<Term>
  where
    Atom: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
  {
    // Acquire write access to the inner table - this always succeeds
    // as our borrows are fully contained within the public API.
    let Some(mut guard) = self.inner.try_write() else {
      unreachable!()
    };

    // Retrieve a mutable reference to the underlying table;
    // bail early if the table was not initialized.
    let Some(data) = guard.as_mut() else {
      return None;
    };

    // Remove the key-value pair from the table.
    data.remove(key)
  }

  #[inline]
  pub(crate) fn clear(&self) -> Vec<(Atom, Term)> {
    // Acquire write access to the inner table - this always succeeds
    // as our borrows are fully contained within the public API.
    let Some(mut guard) = self.inner.try_write() else {
      unreachable!()
    };

    // Retrieve a mutable reference to the underlying table;
    // bail early if the table was not initialized.
    let Some(data) = guard.as_mut() else {
      return Vec::new();
    };

    // Remove all key-value pairs from the table.
    Vec::from_iter(data.drain())
  }

  pub(crate) fn pairs(&self) -> Vec<(Atom, Term)> {
    #[inline]
    fn clone((atom, term): (&Atom, &Term)) -> (Atom, Term) {
      (*atom, term.clone())
    }

    // Acquire read access to the inner table - this always succeeds
    // as our borrows are fully contained within the public API.
    let Some(guard) = self.inner.try_read() else {
      unreachable!()
    };

    // Retrieve an immutable reference to the underlying table;
    // bail early if the table was not initialized.
    let Some(data) = guard.as_ref() else {
      return Vec::new();
    };

    // Collect all key-value pairs from the table as a single list.
    Vec::from_iter(data.iter().map(clone))
  }

  pub(crate) fn keys(&self) -> Vec<Atom> {
    // Acquire read access to the inner table - this always succeeds
    // as our borrows are fully contained within the public API.
    let Some(guard) = self.inner.try_read() else {
      unreachable!()
    };

    // Retrieve an immutable reference to the underlying table;
    // bail early if the table was not initialized.
    let Some(data) = guard.as_ref() else {
      return Vec::new();
    };

    // Collect all keys from the table as a single list.
    Vec::from_iter(data.keys().copied())
  }

  pub(crate) fn values(&self) -> Vec<Term> {
    // Acquire read access to the inner table - this always succeeds
    // as our borrows are fully contained within the public API.
    let Some(guard) = self.inner.try_read() else {
      unreachable!()
    };

    // Retrieve an immutable reference to the underlying table;
    // bail early if the table was not initialized.
    let Some(data) = guard.as_ref() else {
      return Vec::new();
    };

    // Collect all values from the table as a single list.
    Vec::from_iter(data.values().cloned())
  }
}
