//! Global tables for atoms and other runtime data.

mod atom_table;
mod proc_table;

pub(crate) use self::atom_table::AtomTable;
pub(crate) use self::atom_table::AtomTableError;

pub(crate) use self::proc_table::ProcAccessError;
pub(crate) use self::proc_table::ProcInsertError;
pub(crate) use self::proc_table::ProcTable;
pub(crate) use self::proc_table::ProcTableKeys;

pub use self::atom_table::MAX_ATOM_BYTES;
pub use self::atom_table::MAX_ATOM_CHARS;
pub use self::atom_table::MAX_ATOM_COUNT;
