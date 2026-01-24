use tokio::signal::unix;
use tokio::signal::unix::Signal;
use tokio::signal::unix::SignalKind;
use tracing::Level;
use tracing::Span;
use tracing::debug;
use tracing::info;
use tracing::span;

use crate::core::InternalPid;
use crate::erts::Process;
use crate::erts::ProcessFlags;
use crate::init::ExitCode;
use crate::init::Terminate;
use crate::init::TerminateSend;
use crate::raise;

pub(crate) async fn task(quit: TerminateSend) {
  let this: InternalPid = Process::this();
  let span: Span = span!(target: "errai", Level::DEBUG, "signal-handler", %this);

  debug!(target: "errai", parent: &span, "initializing");

  Process::set_flag(ProcessFlags::TRAP_EXIT, true);
  Process::register(this, "$__ERTS_SIGNAL_HANDLER");

  debug!(target: "errai", parent: &span, "activating");

  let mut sigint: Signal = signal(libc::SIGINT);
  let mut sigquit: Signal = signal(libc::SIGQUIT);
  let mut sigterm: Signal = signal(libc::SIGTERM);
  let mut sigusr1: Signal = signal(libc::SIGUSR1);

  debug!(target: "errai", parent: &span, "polling");

  'signal: loop {
    tokio::select! {
      _ = sigint.recv() => {
        info!(target: "errai", parent: &span, "SIGINT received - stopping");
        quit.send(Terminate::Stop(ExitCode::SUCCESS)).await;
        break 'signal;
      }
      _ = sigquit.recv() => {
        info!(target: "errai", parent: &span, "SIGQUIT received - halting");
        quit.send(Terminate::Halt(ExitCode::SUCCESS)).await;
        break 'signal;
      }
      _ = sigterm.recv() => {
        info!(target: "errai", parent: &span, "SIGTERM received - stopping");
        quit.send(Terminate::Stop(ExitCode::SUCCESS)).await;
        break 'signal;
      }
      _ = sigusr1.recv() => {
        info!(target: "errai", parent: &span, "SIGUSR1 received - dumping");
        quit.send(Terminate::Dump("Received SIGUSR1")).await;
        break 'signal;
      }
    }
  }

  debug!(target: "errai", parent: &span, "exiting");
}

#[inline]
fn signal(kind: i32) -> Signal {
  match unix::signal(SignalKind::from_raw(kind)) {
    Ok(signal) => signal,
    Err(error) => raise!(Error, SysInv, error),
  }
}
