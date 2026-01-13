use std::num::NonZeroU64;

use crate::core::WeakProcSend;
use crate::erts::DynMessage;
use crate::lang::Atom;
use crate::lang::Exit;
use crate::lang::InternalDest;
use crate::lang::InternalPid;
use crate::lang::InternalRef;

#[derive(Debug)]
pub(crate) enum Signal {
  Exit {
    from: InternalPid,
    reason: Exit,
    linked: bool,
  },
  Send {
    data: DynMessage,
  },
  Link {
    from: InternalPid,
    send: WeakProcSend,
  },
  Unlink {
    from: InternalPid,
    send: WeakProcSend,
    ulid: NonZeroU64,
  },
  UnlinkAck {
    from: InternalPid,
    ulid: NonZeroU64,
  },
  Monitor {
    from: InternalPid,
    send: WeakProcSend,
    mref: InternalRef,
  },
  Demonitor {
    from: InternalPid,
    mref: InternalRef,
  },
  MonitorDown {
    mref: InternalRef,
    item: InternalDest,
    info: Atom,
  },
}
