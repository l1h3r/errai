//! Errai runtime initialization and lifecycle management

use std::io::Error;
use std::panic;
use std::panic::PanicHookInfo;
use std::process;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;
use tokio::sync::oneshot::Sender;

use crate::bifs;
use crate::erts::Message;
use crate::erts::Process;
use crate::erts::ProcessFlags;
use crate::lang::InternalPid;
use crate::lang::Term;

type PanicHook = Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>;

// A conservative default of the number of CPUs on the running machine.
const DEFAULT_CPUS: usize = 1;

// Default runtime configuration
const DEFAULT_EVENT_INTERVAL: u32 = 61;
const DEFAULT_GLOBAL_QUEUE_INTERVAL: u32 = 31;
const DEFAULT_MAX_BLOCKING_THREADS: usize = 512;
const DEFAULT_MAX_IO_EVENTS_PER_TICK: usize = 1024;
const DEFAULT_THREAD_KEEP_ALIVE: Duration = Duration::from_millis(10 * 1000);
const DEFAULT_THREAD_STACK_SIZE: usize = 2 * 1024 * 1024;

// Error codes used to terminate the running process.
const ERR_CODE_SUCCESS: i32 = 0;
const ERR_CODE_FAILURE_INIT: i32 = -1;
const ERR_CODE_FAILURE_EXEC: i32 = -2;

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
  let hook: PanicHook = panic::take_hook();

  panic::set_hook(Box::new(move |info| {
    eprintln!("[errai]: Unhandled panic: {info:?}");
    hook(info);
  }));

  let runtime: Runtime = match build_runtime() {
    Ok(runtime) => runtime,
    Err(error) => {
      eprintln!("[errai]: Failed to build runtime: {error}");
      process::exit(ERR_CODE_FAILURE_INIT);
    }
  };

  let task = async move {
    // Set up a channel and wait for the main process to tell use when to exit
    let (shutdown, mailbox): (Sender<()>, Receiver<()>) = oneshot::channel();

    // Spawn the root process...
    let _root: InternalPid = bifs::process_spawn_root(async move {
      // We *must* trap exits to ensure we forward shutdown signals to the parent.
      Process::set_flag(ProcessFlags::TRAP_EXIT, true);

      // Register *this* process because it's important...
      Process::register(Process::this(), "$__ERTS_ROOT");

      // Spawn the main process to do whatever the caller wanted to do.
      //
      // The process is linked to *this* process since we rely on the delivery
      // of exit signals for a clean shutdown.
      let _application: InternalPid = Process::spawn_link(future);

      // Block and wait for an exit signal
      match Process::receive::<Term>().await {
        Message::Term(_term) => {
          // Ignore messages here, we poll and drop terms to keep the queue small.
        }
        Message::Exit(sender, reason) => {
          println!("[errai]: Shutdown initialized: {sender} because {reason}");

          // Tell the parent to begin the shutdown process
          if let Err(()) = shutdown.send(()) {
            eprintln!("[errai]: Failed to send application shutdown");
          }
        }
      }
    });

    // Wait for the shutdown message
    mailbox.await
  };

  if let Err(error) = runtime.block_on(task) {
    eprintln!("[errai]: Failed to run application: {error}");
    process::exit(ERR_CODE_FAILURE_EXEC);
  } else {
    process::exit(ERR_CODE_SUCCESS);
  }
}

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
    .event_interval(DEFAULT_EVENT_INTERVAL)
    .global_queue_interval(DEFAULT_GLOBAL_QUEUE_INTERVAL)
    .max_blocking_threads(DEFAULT_MAX_BLOCKING_THREADS)
    .max_io_events_per_tick(DEFAULT_MAX_IO_EVENTS_PER_TICK)
    .thread_keep_alive(DEFAULT_THREAD_KEEP_ALIVE)
    .thread_name_fn(next_worker_name)
    .thread_stack_size(DEFAULT_THREAD_STACK_SIZE)
    .worker_threads(available_cpus())
    .build()
}

fn available_cpus() -> usize {
  match thread::available_parallelism() {
    Ok(count) => count.get(),
    Err(_) => DEFAULT_CPUS,
  }
}

fn next_worker_id() -> u32 {
  WORKER_ID.fetch_add(1, Ordering::SeqCst)
}

fn next_worker_name() -> String {
  format!("errai-worker-{}", next_worker_id())
}
