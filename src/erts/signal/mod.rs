// Signal Handling
//
// # Erlang References
//
// <https://www.erlang.org/doc/system/ref_man_processes#sending-exit-signals>
// <https://www.erlang.org/doc/system/ref_man_processes#receiving-exit-signals>
// <https://www.erlang.org/doc/apps/erts/erl_dist_protocol#link_protocol>

mod control;
mod dynamic;
mod message;
mod trace;
mod traits;

pub(crate) use self::control::ControlSignal;
pub(crate) use self::control::SignalDemonitor;
pub(crate) use self::control::SignalExit;
pub(crate) use self::control::SignalLink;
pub(crate) use self::control::SignalLinkExit;
pub(crate) use self::control::SignalMonitor;
pub(crate) use self::control::SignalMonitorDown;
pub(crate) use self::control::SignalUnlink;
pub(crate) use self::control::SignalUnlinkAck;

pub(crate) use self::dynamic::Signal;

pub(crate) use self::message::MessageSignal;
pub(crate) use self::message::SignalSend;

pub(crate) use self::trace::trace_enter;
pub(crate) use self::trace::trace_leave;
pub(crate) use self::trace::trace_span;

pub(crate) use self::traits::SignalEmit;
pub(crate) use self::traits::SignalRecv;
