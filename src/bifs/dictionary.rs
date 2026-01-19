// -----------------------------------------------------------------------------
// Process Dictionary
//
// BEAM Reference:
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.h
//   https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c
// -----------------------------------------------------------------------------

use crate::core::Atom;
use crate::core::Term;
use crate::proc::ProcTask;

/// Stores a key-value pair in the process dictionary.
///
/// Returns the previous value if the key was already set.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L353>
pub(crate) fn proc_dict_put(this: &ProcTask, key: Atom, value: Term) -> Option<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.insert(key, value)
}

/// Retrieves a value from the process dictionary.
///
/// Returns `None` if the key is not set.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L324>
pub(crate) fn proc_dict_get(this: &ProcTask, key: Atom) -> Option<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.get(&key)
}

/// Deletes a key from the process dictionary.
///
/// Returns the previous value if the key was set.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L373>
pub(crate) fn proc_dict_delete(this: &ProcTask, key: Atom) -> Option<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.remove(&key)
}

/// Clears the entire process dictionary.
///
/// Returns all key-value pairs that were stored.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L363>
pub(crate) fn proc_dict_clear(this: &ProcTask) -> Vec<(Atom, Term)> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.clear()
}

/// Returns all key-value pairs in the process dictionary.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L315>
pub(crate) fn proc_dict_pairs(this: &ProcTask) -> Vec<(Atom, Term)> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.pairs()
}

/// Returns all keys in the process dictionary.
///
/// BEAM Builtin: <https://github.com/erlang/otp/blob/master/erts/emulator/beam/erl_process_dict.c#L333>
pub(crate) fn proc_dict_keys(this: &ProcTask) -> Vec<Atom> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.keys()
}

/// Returns all values in the process dictionary.
///
/// BEAM Builtin: N/A
pub(crate) fn proc_dict_values(this: &ProcTask) -> Vec<Term> {
  debug_assert!(!this.internal.is_locked());
  this.internal.lock().dictionary.values()
}
