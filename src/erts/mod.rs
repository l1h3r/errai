//! Core ERTS (Errai Runtime System) types and APIs.

mod state;

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
pub(crate) use self::state::unbounded_channel;

pub(crate) struct Signal {}
