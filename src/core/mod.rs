mod error;
mod table;
mod types;

pub(crate) use self::error::fatal;
pub(crate) use self::error::raise;

pub(crate) use self::table::AtomTable;
pub(crate) use self::table::AtomTableError;

pub use self::table::MAX_ATOM_BYTES;
pub use self::table::MAX_ATOM_CHARS;
pub use self::table::MAX_ATOM_COUNT;

pub use self::types::Atom;
