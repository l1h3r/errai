//! Core "erts" types of the Errai runtime system.

mod message;
mod process;
mod runtime;
mod signal;
mod spawn;
mod table;

pub(crate) use self::signal::Signal;
pub(crate) use self::table::AtomTable;
pub(crate) use self::table::AtomTableError;

pub use self::message::DynMessage;
pub use self::message::ExitMessage;
pub use self::message::Message;
pub use self::process::Process;
pub use self::runtime::Runtime;
pub use self::spawn::SpawnConfig;
pub use self::spawn::SpawnHandle;
pub use self::table::ProcTable;
pub use self::table::ProcTableFull;
pub use self::table::ProcTableKeys;
