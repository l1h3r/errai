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

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use crate::core::Atom;
  use crate::core::Term;
  use crate::proc::ProcDict;

  #[test]
  fn test_new_dict_is_empty() {
    let dict: ProcDict = ProcDict::new();
    let item: Option<Term> = dict.get(&Atom::new("nonexistent"));

    assert!(item.is_none());
  }

  #[test]
  fn test_lazy_allocation() {
    let dict: ProcDict = ProcDict::new();

    assert!(dict.keys().is_empty());
    assert!(dict.values().is_empty());
    assert!(dict.pairs().is_empty());
  }

  #[test]
  fn test_insert_and_get() {
    let mut dict: ProcDict = ProcDict::new();
    let name: Atom = Atom::new("name");
    let data: Term = Term::new(Atom::new("data"));

    let prev: Option<Term> = dict.insert(name, data.clone());
    assert!(prev.is_none());

    let item: Option<Term> = dict.get(&name);
    assert!(item.is_some());
    assert_eq!(item.unwrap(), data);
  }

  #[test]
  fn test_insert_replaces_old_value() {
    let mut dict: ProcDict = ProcDict::new();
    let name: Atom = Atom::new("name");

    let v1: Term = Term::new(Atom::new("v1"));
    let v2: Term = Term::new(Atom::new("v2"));

    let _old: Option<Term> = dict.insert(name, v1.clone());
    let item: Option<Term> = dict.insert(name, v2.clone());

    assert!(item.is_some());
    assert_eq!(item.unwrap(), v1);
    assert_eq!(dict.get(&name).unwrap(), v2);
  }

  #[test]
  fn test_remove() {
    let mut dict: ProcDict = ProcDict::new();
    let name: Atom = Atom::new("name");
    let data: Term = Term::new(Atom::new("data"));

    dict.insert(name, data.clone());

    let item: Option<Term> = dict.remove(&name);

    assert!(item.is_some());
    assert!(dict.get(&name).is_none());
    assert_eq!(item.unwrap(), data);
  }

  #[test]
  fn test_remove_nonexistent() {
    let mut dict: ProcDict = ProcDict::new();
    let item: Option<Term> = dict.remove(&Atom::new("nonexistent"));

    assert!(item.is_none());
  }

  #[test]
  fn test_clear() {
    let mut dict: ProcDict = ProcDict::new();

    dict.insert(Atom::new("k1"), Term::new(Atom::new("v1")));
    dict.insert(Atom::new("k2"), Term::new(Atom::new("v2")));

    let pairs: Vec<(Atom, Term)> = dict.clear();

    assert_eq!(pairs.len(), 2);
    assert!(dict.get(&Atom::new("k1")).is_none());
    assert!(dict.get(&Atom::new("k2")).is_none());
  }

  #[test]
  fn test_clear_empty_dict() {
    let mut dict: ProcDict = ProcDict::new();
    let pairs: Vec<(Atom, Term)> = dict.clear();

    assert!(pairs.is_empty());
  }

  #[test]
  fn test_keys() {
    let mut dict: ProcDict = ProcDict::new();

    let k1: Atom = Atom::new("k1");
    let k2: Atom = Atom::new("k2");

    dict.insert(k1, Term::new(Atom::EMPTY));
    dict.insert(k2, Term::new(Atom::EMPTY));

    let keys: Vec<Atom> = dict.keys();

    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&k1));
    assert!(keys.contains(&k2));
  }

  #[test]
  fn test_values() {
    let mut dict: ProcDict = ProcDict::new();

    let v1: Term = Term::new(Atom::new("v1"));
    let v2: Term = Term::new(Atom::new("v2"));

    dict.insert(Atom::new("k1"), v1.clone());
    dict.insert(Atom::new("k2"), v2.clone());

    let values: Vec<Term> = dict.values();

    assert_eq!(values.len(), 2);
    assert!(values.contains(&v1));
    assert!(values.contains(&v2));
  }

  #[test]
  fn test_pairs() {
    let mut dict: ProcDict = ProcDict::new();

    let name: Atom = Atom::new("name");
    let data: Term = Term::new(Atom::new("data"));

    dict.insert(name, data.clone());

    let pairs: Vec<(Atom, Term)> = dict.pairs();

    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0], (name, data));
  }

  #[test]
  fn test_multiple_operations() {
    let mut dict: ProcDict = ProcDict::new();

    for index in 0..10 {
      let name: Atom = Atom::new(&format!("name{index}"));
      let data: Term = Term::new(Atom::new(&format!("data{index}")));
      dict.insert(name, data);
    }

    assert_eq!(dict.keys().len(), 10);

    for index in 0..5 {
      dict.remove(&Atom::new(&format!("name{index}")));
    }

    assert_eq!(dict.keys().len(), 5);
  }

  #[test]
  fn test_debug_format_empty() {
    let dict: ProcDict = ProcDict::new();
    let data: String = format!("{:?}", dict);

    assert_eq!(data, "ProcDict {}");
  }

  #[test]
  fn test_debug_format_with_data() {
    let mut dict: ProcDict = ProcDict::new();
    dict.insert(Atom::new("key"), Term::new(Atom::new("value")));

    let data: String = format!("{:?}", dict);

    assert_eq!(data, "ProcDict {key: value}");
  }
}
