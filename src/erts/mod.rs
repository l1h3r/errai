//! Core ERTS (Errai Runtime System) types and APIs.

mod signal;
mod state;

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

pub(crate) use self::state::ProcData;
pub(crate) use self::state::ProcDict;
pub(crate) use self::state::ProcExternal;
pub(crate) use self::state::ProcFlags;
pub(crate) use self::state::ProcInternal;
pub(crate) use self::state::ProcLink;
pub(crate) use self::state::ProcMail;
pub(crate) use self::state::ProcMonitor;
pub(crate) use self::state::ProcReadOnly;
pub(crate) use self::state::ProcRecv;
pub(crate) use self::state::ProcSend;
pub(crate) use self::state::ProcTask;
pub(crate) use self::state::unbounded_channel;
