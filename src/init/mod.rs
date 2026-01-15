//! Errai runtime initialization and lifecycle management

use std::io::Error;
use std::panic;
use std::panic::PanicHookInfo;
use std::process;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::thread;
use tokio::runtime::Builder;
use tokio::runtime::Runtime as TokioRuntime;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;
use tokio::sync::oneshot::Sender;
use tracing::Level;
use tracing::field;
use tracing::subscriber;
use tracing_subscriber::FmtSubscriber;

use crate::bifs;
use crate::core::ProcessFlags;
use crate::erts::DynMessage;
use crate::erts::Process;
use crate::erts::Runtime;
use crate::lang::InternalPid;
use crate::lang::Term;

type PanicHook = Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>;

// Counter used to generate unique names for worker threads
static WORKER_ID: AtomicU32 = AtomicU32::new(1);

/// Runs the given `future` to completion on the Errai runtime system.
///
/// This is the main entrypoint to the Errai runtime.
///
/// Note: This function terminates upon completion!
pub fn block_on<F>(future: F) -> !
where
  F: Future<Output = ()> + Send + 'static,
{
  // ---------------------------------------------------------------------------
  // 1. Configure Tracing
  // ---------------------------------------------------------------------------

  if let Err(error) = subscriber::set_global_default(build_tracing()) {
    eprintln!("Failed to initialize runtime: {error}");
    process::exit(Runtime::E_CODE_FAILURE_INIT);
  }

  // ---------------------------------------------------------------------------
  // 2. Configure Panic Hook
  // ---------------------------------------------------------------------------

  let hook: PanicHook = panic::take_hook();

  // TODO: Avoid logging twice. Maybe don't add this..
  panic::set_hook(Box::new(move |info| {
    tracing::info!(
      location = info.location().map(field::display),
      payload = field::display(Term::new_error_ref(info.payload())),
      "Uncaught exception"
    );

    hook(info);
  }));

  // ---------------------------------------------------------------------------
  // 3. Configure Tokio Runtime
  // ---------------------------------------------------------------------------

  let runtime: TokioRuntime = match build_runtime() {
    Ok(runtime) => runtime,
    Err(error) => {
      tracing::error!(%error, "Failed to initialize runtime");
      process::exit(Runtime::E_CODE_FAILURE_INIT);
    }
  };

  // ---------------------------------------------------------------------------
  // 3. Define Root Task
  // ---------------------------------------------------------------------------

  let task = async move {
    let (send, recv): (Sender<()>, Receiver<()>) = oneshot::channel();

    spawn_root_process(future, send);

    recv.await
  };

  // ---------------------------------------------------------------------------
  // 4. Run
  // ---------------------------------------------------------------------------

  if let Err(error) = runtime.block_on(task) {
    tracing::error!(%error, "Failed to execute runtime");
    process::exit(Runtime::E_CODE_FAILURE_EXEC);
  }

  // ---------------------------------------------------------------------------
  // 5. Shutdown & Exit
  // ---------------------------------------------------------------------------

  runtime.shutdown_timeout(Runtime::SHUTDOWN_TIMEOUT);

  process::exit(Runtime::E_CODE_SUCCESS);
}

fn build_tracing() -> FmtSubscriber {
  FmtSubscriber::builder()
    .log_internal_errors(true)
    .with_ansi(true)
    .with_level(true)
    .with_line_number(true)
    .with_max_level(Level::TRACE)
    .with_target(true)
    .with_thread_ids(true)
    .with_thread_names(true)
    .finish()
}

fn build_runtime() -> Result<TokioRuntime, Error> {
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
    .event_interval(Runtime::DEFAULT_EVENT_INTERVAL)
    .global_queue_interval(Runtime::DEFAULT_GLOBAL_QUEUE_INTERVAL)
    .max_blocking_threads(Runtime::DEFAULT_MAX_BLOCKING_THREADS)
    .max_io_events_per_tick(Runtime::DEFAULT_MAX_IO_EVENTS_PER_TICK)
    .thread_keep_alive(Runtime::DEFAULT_THREAD_KEEP_ALIVE)
    .thread_name_fn(next_worker_name)
    .thread_stack_size(Runtime::DEFAULT_THREAD_STACK_SIZE)
    .worker_threads(available_cpus())
    .build()
}

fn available_cpus() -> usize {
  match thread::available_parallelism() {
    Ok(count) => count.get(),
    Err(_) => Runtime::DEFAULT_PARALLELISM,
  }
}

fn next_worker_id() -> u32 {
  WORKER_ID.fetch_add(1, Ordering::SeqCst)
}

fn next_worker_name() -> String {
  format!("errai-worker-{}", next_worker_id())
}

fn spawn_root_process<F>(future: F, shutdown: Sender<()>) -> InternalPid
where
  F: Future<Output = ()> + Send + 'static,
{
  bifs::process_spawn_root(async move {
    let this: InternalPid = Process::this();

    tracing::trace!(pid = %this, "Enter Root Process");

    // We *must* trap exits to ensure we forward shutdown signals to the parent.
    Process::set_flag(ProcessFlags::TRAP_EXIT, true);

    // Register *this* process because it's important...
    Process::register(this, "$__ERTS_ROOT");

    tracing::trace!(pid = %this, "Spawn Application Process");

    // Spawn the main process to do whatever the caller wanted to do.
    //
    // The process is linked to *this* process since we rely on the delivery
    // of exit signals for a clean shutdown.
    let application: InternalPid = Process::spawn_link(future);

    tracing::trace!(pid = %this, "Start Runloop");

    // Block and wait for an exit signal
    'run: loop {
      match Process::receive_any().await {
        DynMessage::Term(_term) => {
          // Ignore messages here, we poll and drop terms to keep the queue small.
        }
        DynMessage::Exit(exit) if exit.from() == application => {
          tracing::info!(?exit, "Runtime shutdown initialized");
          break 'run;
        }
        DynMessage::Exit(_exit) => {
          // We only care about exit messages sent from the app process.
        }
        DynMessage::Down(_down) => {
          // We don't care about monitor messages.
        }
      }
    }

    tracing::trace!(pid = %this, "Exit Runloop");

    if let Err(()) = shutdown.send(()) {
      tracing::error!("Failed to shut down runtime: channel closed");
      process::exit(Runtime::E_CODE_FAILURE_EXEC);
    }
  })
}
