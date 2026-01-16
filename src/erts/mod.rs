//! Core "erts" types of the Errai runtime system.

mod message;
mod process;
mod runtime;
mod signal;
mod spawn;

pub(crate) use self::signal::ControlSignal;
pub(crate) use self::signal::MessageSignal;
pub(crate) use self::signal::Signal;
pub(crate) use self::signal::SignalDemonitor;
pub(crate) use self::signal::SignalEmit;
pub(crate) use self::signal::SignalExit;
pub(crate) use self::signal::SignalLink;
pub(crate) use self::signal::SignalLinkExit;
pub(crate) use self::signal::SignalMonitor;
pub(crate) use self::signal::SignalMonitorDown;
pub(crate) use self::signal::SignalRecv;
pub(crate) use self::signal::SignalSend;
pub(crate) use self::signal::SignalUnlink;
pub(crate) use self::signal::SignalUnlinkAck;

pub use self::message::DownMessage;
pub use self::message::DynMessage;
pub use self::message::ExitMessage;
pub use self::message::Message;
pub use self::process::Process;
pub use self::process::ProcessFlags;
pub use self::process::ProcessInfo;
pub use self::runtime::Runtime;
pub use self::spawn::SpawnConfig;
pub use self::spawn::SpawnHandle;
