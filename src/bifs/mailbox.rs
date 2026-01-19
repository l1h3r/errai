// -----------------------------------------------------------------------------
// Process Mailbox
// -----------------------------------------------------------------------------

use crate::bifs;
use crate::core::ExternalDest;
use crate::core::InternalPid;
use crate::core::Term;
use crate::erts::DynMessage;
use crate::erts::Message;
use crate::proc::ProcMail;
use crate::proc::ProcTask;
use crate::raise;

/// Sends a message to the given destination.
///
/// # Destination Handling
///
/// - **InternalProc**: Sends directly to the PID
/// - **InternalName**: Resolves name, then sends (raises if unregistered)
/// - **ExternalProc/ExternalName**: Currently unimplemented
///
/// # Error Handling
///
/// - Silently drops messages to dead PIDs
/// - Raises exception for unregistered names
pub(crate) fn proc_send(this: &ProcTask, dest: ExternalDest, term: Term) {
  match dest {
    ExternalDest::InternalProc(pid) => {
      bifs::proc_with(this, pid, |proc| {
        let Some(proc) = proc else {
          return; // Never fail when sending to a non-existent PID.
        };

        proc.readonly.send_message(this.readonly.mpid, term);
      });
    }
    ExternalDest::InternalName(name) => {
      let Some(pid) = bifs::proc_whereis(name) else {
        raise!(Error, BadArg, "unregistered name");
      };

      bifs::proc_with(this, pid, |proc| {
        let Some(proc) = proc else {
          raise!(Error, BadArg, "unregistered name");
        };

        proc.readonly.send_message(this.readonly.mpid, term);
      });
    }
    ExternalDest::ExternalProc(pid) => {
      todo!("Handle External PID Send - {pid}")
    }
    ExternalDest::ExternalName(name, node) => {
      todo!("Handle External Name Send - {name} @ {node}")
    }
  }
}

/// Receives a message matching type `T` from the mailbox.
///
/// Wraps the message in a [`Message`] envelope that may also contain
/// EXIT or DOWN signals.
pub(crate) async fn proc_receive<T>(pid: InternalPid) -> Message<Box<T>>
where
  T: 'static,
{
  ProcMail::receive::<T>(pid).await
}

/// Receives an exact message of type `T` from the mailbox.
///
/// Returns the unwrapped value without the [`Message`] envelope.
pub(crate) async fn proc_receive_exact<T>(pid: InternalPid) -> Box<T>
where
  T: 'static,
{
  ProcMail::receive_exact::<T>(pid).await
}

/// Receives any message from the mailbox without filtering.
pub(crate) async fn proc_receive_any(pid: InternalPid) -> DynMessage {
  ProcMail::receive_any(pid).await
}

/// Receives a message matching a custom filter function.
///
/// The filter is called for each message until one matches.
pub(crate) async fn proc_receive_dyn<F>(pid: InternalPid, filter: F) -> DynMessage
where
  F: Fn(&DynMessage) -> bool,
{
  ProcMail::receive_dyn(pid, filter).await
}
