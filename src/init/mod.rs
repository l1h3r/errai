mod kernel;
mod server;
mod shutdown;

pub(crate) use self::shutdown::ExitCode;
pub(crate) use self::shutdown::Terminate;
pub(crate) use self::shutdown::TerminateRecv;
pub(crate) use self::shutdown::TerminateSend;

use std::fmt::Display;
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::runtime::Runtime as TokioRuntime;
use tracing::Level;
use tracing::Span;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::span;

use crate::bifs;
use crate::core::InternalPid;
use crate::error::Exception;
use crate::error::ExceptionClass;
use crate::error::ExceptionGroup;
use crate::erts::RuntimeConfig;
use crate::loom::sync::atomic::AtomicBool;
use crate::loom::sync::atomic::AtomicU32;
use crate::loom::sync::atomic::Ordering;
use crate::node::LocalNode;
use crate::raise;
use crate::utils::measure_fn;

static INIT: AtomicBool = AtomicBool::new(false);

/// Runs the given future to completion on the Errai runtime system.
///
/// This is the same as calling `run_opts(future, Default::default())`.
#[inline]
pub fn run<F>(future: F) -> !
where
  F: Future<Output = ()> + Send + 'static,
{
  run_opts(future, Default::default())
}

/// Runs the given future to completion on the Errai runtime system.
pub fn run_opts<F>(future: F, config: RuntimeConfig) -> !
where
  F: Future<Output = ()> + Send + 'static,
{
  if INIT.swap(true, Ordering::SeqCst) {
    raise!(Error, SysInv, "Errai is already running!");
  }

  if let Err(error) = init_tracing_subsriber(&config) {
    eprintln!("failed to set tracing subscriber:");
    eprintln!("    {}", error.error());
  }

  // SAFETY: We ensured above that `INIT` was not already set.
  unsafe { run_unchecked(future, config) }
}

unsafe fn run_unchecked<F>(future: F, config: RuntimeConfig) -> !
where
  F: Future<Output = ()> + Send + 'static,
{
  let span: Span = span!(target: "errai", Level::DEBUG, "init::run");

  let runtime: TokioRuntime = match build_tokio_runtime(&config) {
    Ok(runtime) => runtime,
    Err(error) => {
      error!(
        target: "errai",
        parent: &span,
        error = error.error(),
        "failed to build runtime",
      );

      ExitCode::FAILURE.exit_process();
    }
  };

  let terminate: Terminate = runtime.block_on(async {
    debug!(target: "errai", parent: &span, "initializing");

    let (send, mut recv): (TerminateSend, TerminateRecv) = shutdown::channel();

    let _ignore: &LocalNode = LocalNode::this();
    let _ignore: InternalPid = bifs::proc_spawn_root(kernel::task(future, send));

    debug!(target: "errai", parent: &span, "polling");

    let terminate: Option<Terminate> = recv.recv().await;

    debug!(target: "errai", parent: &span, "exiting");

    if let Some(result) = terminate {
      result
    } else {
      raise!(Error, SysInv, "shutdown channel closed");
    }
  });

  match terminate {
    Terminate::Stop(ecode) => {
      info!(
        target: "errai",
        parent: &span,
        status = ecode.to_i32(),
        timeout = ?config.rt_shutdown_timeout,
        "system stopping",
      );

      let elapsed: Duration = measure_fn(|| {
        runtime.shutdown_timeout(config.rt_shutdown_timeout);
      });

      info!(
        target: "errai",
        parent: &span,
        elapsed = ?elapsed,
        "system stopped",
      );

      ecode.exit_process();
    }
    Terminate::Halt(ecode) => {
      info!(
        target: "errai",
        parent: &span,
        status = ecode.to_i32(),
        "system halted",
      );

      ecode.exit_process();
    }
    Terminate::Dump(slogan) => {
      info!(
        target: "errai",
        parent: &span,
        status = ExitCode::FAILURE.to_i32(),
        "system halted",
      );

      // TODO: Crash Dump

      ExitCode::FAILURE.exit_process();
    }
  }
}

/// Builds the global tracing subscriber configuration.
#[cfg(feature = "tracing")]
fn init_tracing_subsriber(config: &RuntimeConfig) -> Result<(), Exception> {
  use tracing_subscriber::FmtSubscriber;
  use tracing_subscriber::fmt::format;
  use tracing_subscriber::util::SubscriberInitExt;

  FmtSubscriber::builder()
    .event_format(format().compact())
    .log_internal_errors(true)
    .with_ansi(true)
    .with_file(config.tracing_source_file)
    .with_level(true)
    .with_line_number(config.tracing_source_line)
    .with_max_level(config.tracing_filter())
    .with_target(config.tracing_source_name)
    .with_thread_ids(config.tracing_thread_info)
    .with_thread_names(config.tracing_thread_info)
    .finish()
    .try_init()
    .map_err(error)
}

#[cfg(not(feature = "tracing"))]
fn init_tracing_subsriber(_config: &RuntimeConfig) -> Result<(), Exception> {
  Ok(())
}

/// Builds the Tokio multi-threaded runtime with the given configuration.
fn build_tokio_runtime(config: &RuntimeConfig) -> Result<TokioRuntime, Exception> {
  // TODO: Maybe try and make use of the following hooks:
  //   - on_after_task_poll
  //   - on_before_task_poll
  //   - on_task_spawn
  //   - on_task_terminate
  //   - on_thread_park
  //   - on_thread_start
  //   - on_thread_stop
  //   - on_thread_unpark
  Builder::new_multi_thread()
    .enable_io()
    .enable_time()
    .event_interval(config.rt_event_interval)
    .global_queue_interval(config.rt_global_queue_interval)
    .max_blocking_threads(config.rt_max_blocking_threads)
    .max_io_events_per_tick(config.rt_max_io_events_per_tick)
    .thread_keep_alive(config.rt_thread_keep_alive)
    .thread_name_fn(next_worker_name)
    .thread_stack_size(config.rt_thread_stack_size)
    .worker_threads(config.rt_worker_threads)
    .build()
    .map_err(error)
}

/// Generates a unique name for the next worker thread.
#[inline]
fn next_worker_name() -> String {
  format!("erts-worker-{:0>2}", next_worker_id())
}

/// Atomically increments and returns the next worker thread ID.
#[inline]
fn next_worker_id() -> u32 {
  static ID: AtomicU32 = AtomicU32::new(1);
  ID.fetch_add(1, Ordering::Relaxed)
}

/// Returns a generic `SysInv` exception with the given error message.
#[cold]
fn error<E>(error: E) -> Exception
where
  E: Display,
{
  Exception::new(ExceptionClass::Error, ExceptionGroup::SysInv, error)
}
