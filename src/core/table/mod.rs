//! Global tables for atoms and other runtime data.

mod atom_table;

pub(crate) use self::atom_table::AtomTable;
pub(crate) use self::atom_table::AtomTableError;

pub use self::atom_table::MAX_ATOM_BYTES;
pub use self::atom_table::MAX_ATOM_CHARS;
pub use self::atom_table::MAX_ATOM_COUNT;
