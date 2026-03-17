use tracing::Level;
use tracing::Span;
use tracing::debug;
use tracing::info;
use tracing::span;

use crate::core::Exit;
use crate::core::InternalPid;
use crate::core::MonitorRef;
use crate::erts::DynMessage;
use crate::erts::Process;
use crate::erts::ProcessFlags;
use crate::init::ExitCode;
use crate::init::Terminate;
use crate::init::TerminateSend;
use crate::init::server;

pub(crate) async fn task<F>(future: F, quit: TerminateSend)
where
  F: Future<Output = ()> + Send + 'static,
{
  let this: InternalPid = Process::this();
  let span: Span = span!(target: "errai", Level::DEBUG, "kernel", %this);

  debug!(target: "errai", parent: &span, "initializing");

  Process::set_flag(ProcessFlags::TRAP_EXIT, true);
  Process::register(this, "$__ERTS_KERNEL");

  debug!(target: "errai", parent: &span, "activating");

  let signal_handler: (InternalPid, MonitorRef) = spawn_signal_handler(quit.clone());
  let app_supervisor: InternalPid = spawn_app_supervisor(future);

  debug!(target: "errai", parent: &span, "polling");

  let terminators: &[InternalPid] = &[app_supervisor];

  'run: loop {
    match Process::receive_any().await {
      DynMessage::Exit(exit) if terminators.contains(&exit.from()) => {
        debug!(
          target: "errai",
          parent: &span,
          from = %exit.from(),
          exit = %exit.exit(),
          "EXIT received",
        );

        // Kill the signal server because:
        // 1. The processes are not linked and we're terminating
        // 2. The runtime will shut down, so there's no value in more signals
        Process::exit(signal_handler.0, Exit::KILLED);

        quit.send(Terminate::Stop(exit_to_code(exit.exit()))).await;

        break 'run;
      }
      DynMessage::Down(down) if *down.mref() == signal_handler.1 => {
        info!(
          target: "errai",
          parent: &span,
          mref = %down.mref(),
          item = %down.item(),
          info = %down.info(),
          "DOWN received",
        );

        // Exit immediately, `signal_handler` sends the termination request.
        break 'run;
      }
      DynMessage::Term(_term) => {
        // Discard regular messages to prevent queue buildup
      }
      DynMessage::Exit(_exit) => {
        // Ignore EXIT from other processes
      }
      DynMessage::Down(_down) => {
        // Ignore DOWN from other processes
      }
    }
  }

  debug!(target: "errai", parent: &span, "exiting");
}

#[inline]
fn spawn_signal_handler(quit: TerminateSend) -> (InternalPid, MonitorRef) {
  Process::spawn_monitor(server::signal_handler::task(quit))
}

#[inline]
fn spawn_app_supervisor<F>(future: F) -> InternalPid
where
  F: Future<Output = ()> + Send + 'static,
{
  Process::spawn_link(server::app_supervisor::task(future))
}

#[inline]
fn exit_to_code(exit: &Exit) -> ExitCode {
  if exit.is_normal() {
    ExitCode::SUCCESS
  } else {
    ExitCode::FAILURE
  }
}
