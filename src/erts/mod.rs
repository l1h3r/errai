//! Core "erts" types of the Errai runtime system.

mod mailbox;
mod process;
mod runtime;
mod spawn;

pub(crate) use self::mailbox::ProcessRecv;
pub(crate) use self::mailbox::ProcessSend;
pub(crate) use self::mailbox::Signal;
pub(crate) use self::mailbox::unbounded_channel;
pub(crate) use self::process::ProcessData;
pub(crate) use self::process::ProcessDict;
pub(crate) use self::process::ProcessSlot;
pub(crate) use self::process::ProcessTask;

pub use self::mailbox::DynMessage;
pub use self::mailbox::ExitMessage;
pub use self::mailbox::Message;
pub use self::process::AliasRef;
pub use self::process::MonitorRef;
pub use self::process::Process;
pub use self::process::ProcessFlags;
pub use self::process::ProcessId;
pub use self::process::ProcessInfo;
pub use self::process::ProcessTable;
pub use self::process::ProcessTableFull;
pub use self::process::TimerRef;
pub use self::runtime::Runtime;
pub use self::spawn::SpawnConfig;
pub use self::spawn::SpawnHandle;
