//! Errai runtime initialization and lifecycle management

use std::io::Error;
use std::panic;
use std::panic::PanicHookInfo;
use std::process;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use tokio::runtime::Builder;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;
use tokio::sync::oneshot::Sender;
use tracing::Level;
use tracing::field;
use tracing::subscriber;
use tracing_subscriber::FmtSubscriber;

use crate::bifs;
use crate::consts;
use crate::core::InternalPid;
use crate::core::Term;
use crate::erts::DynMessage;
use crate::erts::Process;
use crate::erts::ProcessFlags;
use crate::erts::System;

type PanicHook = Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>;

/// Counter for generating unique worker thread names.
static WORKER_ID: AtomicU32 = AtomicU32::new(1);

/// Runs the given future to completion on the Errai runtime system.
///
/// This function initializes the runtime, executes the provided future as
/// a supervised process, and performs graceful shutdown. It is the sole
/// entrypoint to the Errai system.
///
/// # Process Exit
///
/// This function **never returns**. It calls [`process::exit()`] with one
/// of the following codes:
///
/// - `0`: Normal termination (application completed successfully)
/// - `-1`: Initialization failed (tracing, runtime creation)
/// - `-2`: Execution failed (runtime or shutdown error)
///
/// # Initialization
///
/// The runtime performs these initialization steps:
///
/// 1. Configures global tracing subscriber
/// 2. Installs panic hook for exception logging
/// 3. Builds Tokio multi-threaded runtime
/// 4. Spawns root supervisor process
/// 5. Spawns application process (your future)
///
/// # Shutdown
///
/// When the application process terminates, the root supervisor initiates
/// shutdown:
///
/// 1. Receives exit signal from application
/// 2. Signals runtime to stop
/// 3. Waits up to [`SHUTDOWN_TIMEOUT`] for cleanup
/// 4. Exits process with appropriate code
///
/// # Panics
///
/// Panics during initialization cause the process to exit with code `-1`.
/// Panics during execution are logged but handled by the supervision tree.
///
/// [`SHUTDOWN_TIMEOUT`]: consts::SHUTDOWN_TIMEOUT
pub fn block_on<F>(future: F) -> !
where
  F: Future<Output = ()> + Send + 'static,
{
  // ---------------------------------------------------------------------------
  // 1. Configure Tracing
  // ---------------------------------------------------------------------------

  if let Err(error) = subscriber::set_global_default(build_tracing()) {
    eprintln!("Failed to initialize runtime: {error}");
    process::exit(consts::E_CODE_FAILURE_INIT);
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

  let runtime: Runtime = match build_runtime() {
    Ok(runtime) => runtime,
    Err(error) => {
      tracing::error!(%error, "Failed to initialize runtime");
      process::exit(consts::E_CODE_FAILURE_INIT);
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
    process::exit(consts::E_CODE_FAILURE_EXEC);
  }

  // ---------------------------------------------------------------------------
  // 5. Shutdown & Exit
  // ---------------------------------------------------------------------------

  runtime.shutdown_timeout(consts::SHUTDOWN_TIMEOUT);

  process::exit(consts::E_CODE_SUCCESS);
}

/// Builds the global tracing subscriber configuration.
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

/// Builds the Tokio multi-threaded runtime with Errai configuration.
fn build_runtime() -> Result<Runtime, Error> {
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
    .event_interval(consts::DEFAULT_EVENT_INTERVAL)
    .global_queue_interval(consts::DEFAULT_GLOBAL_QUEUE_INTERVAL)
    .max_blocking_threads(consts::DEFAULT_MAX_BLOCKING_THREADS)
    .max_io_events_per_tick(consts::DEFAULT_MAX_IO_EVENTS_PER_TICK)
    .thread_keep_alive(consts::DEFAULT_THREAD_KEEP_ALIVE)
    .thread_name_fn(next_worker_name)
    .thread_stack_size(consts::DEFAULT_THREAD_STACK_SIZE)
    .worker_threads(System::available_cpus())
    .build()
}

/// Atomically increments and returns the next worker thread ID.
fn next_worker_id() -> u32 {
  WORKER_ID.fetch_add(1, Ordering::Relaxed)
}

/// Generates a unique name for the next worker thread.
///
/// Thread names follow the pattern: `errai-worker-N` where N is a
/// monotonically increasing counter.
fn next_worker_name() -> String {
  format!("errai-worker-{}", next_worker_id())
}

/// Spawns the root supervisor process.
///
/// The root process supervises the application process and initiates
/// shutdown when it terminates.
///
/// # Panics
///
/// Exits the process if the shutdown channel is closed unexpectedly,
/// indicating a critical runtime failure.
fn spawn_root_process<F>(future: F, shutdown: Sender<()>) -> InternalPid
where
  F: Future<Output = ()> + Send + 'static,
{
  bifs::proc_spawn_root(async move {
    let this: InternalPid = Process::this();

    tracing::trace!(pid = %this, "Enter Root Process");

    // Trap exit signals to detect application termination
    Process::set_flag(ProcessFlags::TRAP_EXIT, true);

    // Register for identification and debugging
    Process::register(this, "$__ERTS_ROOT");

    tracing::trace!(pid = %this, "Spawn Application Process");

    // Spawn the application process with a link for supervision
    let application: InternalPid = Process::spawn_link(future);

    tracing::trace!(pid = %this, "Start Runloop");

    // Wait for application exit signal
    'run: loop {
      match Process::receive_any().await {
        DynMessage::Term(_term) => {
          // Discard regular messages to prevent queue buildup
        }
        DynMessage::Exit(exit) if exit.from() == application => {
          tracing::info!(?exit, "Runtime shutdown initialized");
          break 'run;
        }
        DynMessage::Exit(_exit) => {
          // Ignore exits from other processes
        }
        DynMessage::Down(_down) => {
          // Ignore monitor down messages
        }
      }
    }

    tracing::trace!(pid = %this, "Exit Runloop");

    if let Err(()) = shutdown.send(()) {
      tracing::error!("Failed to shut down runtime: channel closed");
      process::exit(consts::E_CODE_FAILURE_EXEC);
    }
  })
}
