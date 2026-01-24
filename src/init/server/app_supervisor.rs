use tracing::Level;
use tracing::Span;
use tracing::debug;
use tracing::info;
use tracing::span;

use crate::core::InternalPid;
use crate::erts::DynMessage;
use crate::erts::Process;
use crate::erts::ProcessFlags;

pub(crate) async fn task<F>(future: F)
where
  F: Future<Output = ()> + Send + 'static,
{
  let this: InternalPid = Process::this();
  let span: Span = span!(target: "errai", Level::DEBUG, "app-supervisor", %this);

  debug!(target: "errai", parent: &span, "initializing");

  Process::set_flag(ProcessFlags::TRAP_EXIT, true);
  Process::register(this, "$__ERTS_APP_SUPERVISOR");

  debug!(target: "errai", parent: &span, "activating");

  // TODO: Dont allow removing this link
  let _ignore: InternalPid = Process::spawn_link(future);

  debug!(target: "errai", parent: &span, "polling");

  'run: loop {
    match Process::receive_any().await {
      DynMessage::Term(_term) => {
        // Discard regular messages to prevent queue buildup
      }
      DynMessage::Exit(exit) => {
        info!(
          target: "errai",
          parent: &span,
          from = %exit.from(),
          exit = %exit.exit(),
          "EXIT received",
        );

        break 'run;
      }
      DynMessage::Down(_down) => {
        // Ignore DOWN from other processes
      }
    }
  }

  debug!(target: "errai", parent: &span, "exiting");
}
