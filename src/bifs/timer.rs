// -----------------------------------------------------------------------------
// Process Timers
// -----------------------------------------------------------------------------

use crossbeam_utils::CachePadded;
use hashbrown::HashMap;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use tokio::task::JoinHandle;
use tokio_util::future::FutureExt;
use tokio_util::time::DelayQueue;
use tokio_util::time::delay_queue::Expired;
use tokio_util::time::delay_queue::Key as QueueKey;

use crate::bifs;
use crate::core::InternalDest;
use crate::core::InternalPid;
use crate::core::Term;
use crate::core::TimerRef;
use crate::node::LocalNode;
use crate::proc::ProcTask;
use crate::raise;

// -----------------------------------------------------------------------------
// Read Timer Message
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ReadTimerAck {
  tref: TimerRef,
  info: Option<Duration>,
}

impl ReadTimerAck {
  #[inline]
  pub const fn tref(&self) -> &TimerRef {
    &self.tref
  }

  #[inline]
  pub const fn info(&self) -> Option<Duration> {
    self.info
  }
}

// -----------------------------------------------------------------------------
// Stop Timer Message
// -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct StopTimerAck {
  tref: TimerRef,
  info: Option<Duration>,
}

impl StopTimerAck {
  #[inline]
  pub const fn tref(&self) -> &TimerRef {
    &self.tref
  }

  #[inline]
  pub const fn info(&self) -> Option<Duration> {
    self.info
  }
}

// -----------------------------------------------------------------------------
// BIFs
// -----------------------------------------------------------------------------

pub(crate) fn proc_timer_init(
  this: &ProcTask,
  dest: InternalDest,
  term: Term,
  time: Duration,
) -> TimerRef {
  let tref: TimerRef = TimerRef::new();
  let slot: usize = TimerService::init(this.readonly.mpid, dest, term, tref, time);

  tracing::trace!(
    from = %this.readonly.mpid,
    tref = %tref,
    slot = slot,
    "timer init requested"
  );

  tref
}

pub(crate) fn proc_timer_read(this: &ProcTask, tref: TimerRef) {
  let slot: usize = TimerService::read(tref, this.readonly.mpid);

  tracing::trace!(
    from = %this.readonly.mpid,
    tref = %tref,
    slot = slot,
    mode = "non_blocking",
    "timer read requested"
  );
}

pub(crate) async fn proc_timer_read_blocking(tref: TimerRef) -> Option<Duration> {
  let (send, recv): _ = oneshot::channel();
  let slot: usize = TimerService::read(tref, send);

  tracing::trace!(
    tref = %tref,
    slot = slot,
    mode = "blocking",
    "timer read requested"
  );

  match recv.await {
    Ok(result) => result,
    Err(error) => raise!(Error, SysInv, error),
  }
}

pub(crate) fn proc_timer_stop(this: &ProcTask, tref: TimerRef) {
  let slot: usize = TimerService::stop(tref, this.readonly.mpid);

  tracing::trace!(
    from = %this.readonly.mpid,
    tref = %tref,
    slot = slot,
    mode = "non_blocking",
    "timer stop requested"
  );
}

pub(crate) async fn proc_timer_stop_blocking(tref: TimerRef) -> Option<Duration> {
  let (send, recv): _ = oneshot::channel();
  let slot: usize = TimerService::stop(tref, send);

  tracing::trace!(
    tref = %tref,
    slot = slot,
    mode = "blocking",
    "timer stop requested"
  );

  match recv.await {
    Ok(result) => result,
    Err(error) => raise!(Error, SysInv, error),
  }
}

// -----------------------------------------------------------------------------
// Timer Service
// -----------------------------------------------------------------------------

#[repr(C)]
pub(crate) struct TimerService {
  senders: Vec<CachePadded<UnboundedSender<Signal>>>,
  handles: Vec<JoinHandle<()>>,
}

impl TimerService {
  #[inline]
  pub(crate) fn new(workers: usize) -> Self {
    let mut senders: Vec<CachePadded<UnboundedSender<Signal>>> = Vec::with_capacity(workers);
    let mut handles: Vec<JoinHandle<()>> = Vec::with_capacity(workers);

    tracing::info!(workers, "initializing wheel workers");

    for id in 0..workers {
      let (send, recv): _ = mpsc::unbounded_channel();

      let task: JoinHandle<()> = tokio::spawn(WheelWorker::task(id, recv));

      tracing::debug!(id, "spawned wheel worker");

      senders.push(CachePadded::new(send));
      handles.push(task);
    }

    Self { senders, handles }
  }

  pub(crate) async fn shutdown(self, timeout: Duration) {
    for (id, send) in self.senders.iter().enumerate() {
      if let Err(error) = send.send(Signal::Quit) {
        tracing::warn!(id, ?error, "dangling wheel worker");
      }
    }

    for (id, handle) in self.handles.into_iter().enumerate() {
      match handle.timeout(timeout).await {
        Ok(Ok(())) => {
          // clean shutdown
        }
        Ok(Err(error)) => {
          tracing::error!(id, ?error, "wheel worker join error");
        }
        Err(error) => {
          tracing::error!(id, ?error, "wheel worker timeout error");
        }
      }
    }
  }

  fn init(
    from: InternalPid,
    dest: InternalDest,
    term: Term,
    tref: TimerRef,
    time: Duration,
  ) -> usize {
    let data: InitTimer = InitTimer {
      from,
      dest,
      term,
      tref,
      ends: Instant::now() + time,
    };

    Self::dispatch(tref, Signal::Init(data))
  }

  #[inline]
  fn read(tref: TimerRef, mode: impl Into<ReplyMode>) -> usize {
    let data: ReadTimer = ReadTimer {
      tref,
      mode: mode.into(),
    };

    Self::dispatch(tref, Signal::Read(data))
  }

  #[inline]
  fn stop(tref: TimerRef, mode: impl Into<ReplyMode>) -> usize {
    let data: StopTimer = StopTimer {
      tref,
      mode: mode.into(),
    };

    Self::dispatch(tref, Signal::Stop(data))
  }

  fn dispatch(tref: TimerRef, signal: Signal) -> usize {
    let this: &'static Self = LocalNode::timer();
    let slot: usize = (tref.global_id() % this.senders.len() as u64) as usize;

    if let Err(error) = this.senders[slot].send(signal) {
      raise!(Error, SysInv, error);
    }

    slot
  }
}

// -----------------------------------------------------------------------------
// Timer Signal
// -----------------------------------------------------------------------------

/// Timer operation sent to a wheel worker.
enum Signal {
  /// Initialize a new timer
  Init(InitTimer),
  /// Read timer information
  Read(ReadTimer),
  /// Stop an existing timer
  Stop(StopTimer),
  /// Graceful shutdown signal
  Quit,
}

// -----------------------------------------------------------------------------
// Timer Signal - Init
// -----------------------------------------------------------------------------

/// Signal to initialize a new timer.
struct InitTimer {
  from: InternalPid,
  dest: InternalDest,
  term: Term,
  tref: TimerRef,
  ends: Instant,
}

// -----------------------------------------------------------------------------
// Timer Signal - Read
// -----------------------------------------------------------------------------

/// Signal to read a timer.
struct ReadTimer {
  tref: TimerRef,
  mode: ReplyMode,
}

// -----------------------------------------------------------------------------
// Timer Signal - Stop
// -----------------------------------------------------------------------------

/// Signal to stop a timer.
struct StopTimer {
  tref: TimerRef,
  mode: ReplyMode,
}

// -----------------------------------------------------------------------------
// Reply Mode
// -----------------------------------------------------------------------------

/// Reply mode for timer operations.
enum ReplyMode {
  /// Fast path: direct reply via oneshot channel
  Oneshot(Sender<Option<Duration>>),
  /// Mailbox path: reply via process signal queue
  Mailbox(InternalPid),
}

impl From<Sender<Option<Duration>>> for ReplyMode {
  #[inline]
  fn from(other: Sender<Option<Duration>>) -> Self {
    Self::Oneshot(other)
  }
}

impl From<InternalPid> for ReplyMode {
  #[inline]
  fn from(other: InternalPid) -> Self {
    Self::Mailbox(other)
  }
}

// -----------------------------------------------------------------------------
// Timer State
// -----------------------------------------------------------------------------

struct TimerState {
  from: InternalPid,
  dest: InternalDest,
  term: Term,
  time: Instant,
  qkey: QueueKey,
}

impl TimerState {
  #[inline]
  fn remaining(&self) -> Option<Duration> {
    self.time.checked_duration_since(Instant::now())
  }
}

// -----------------------------------------------------------------------------
// Stream -> Future
// -----------------------------------------------------------------------------

#[repr(transparent)]
struct PollExpired<'a, T>(&'a mut DelayQueue<T>);

impl<'a, T> Future for PollExpired<'a, T> {
  type Output = Option<Expired<T>>;

  #[inline]
  fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
    self.0.poll_expired(context)
  }
}

// -----------------------------------------------------------------------------
// Wheel Worker Task
// -----------------------------------------------------------------------------

/// Task-local state of a wheel worker.
struct WheelWorker {
  id: usize,
  queue: DelayQueue<TimerRef>,
  cache: HashMap<TimerRef, TimerState>,
}

impl WheelWorker {
  #[inline]
  fn new(id: usize) -> Self {
    Self {
      id,
      queue: DelayQueue::new(),
      cache: HashMap::new(),
    }
  }

  async fn task(id: usize, mut recv: UnboundedReceiver<Signal>) {
    let mut this: Self = Self::new(id);

    tracing::trace!(id, "wheel worker started");

    'work: loop {
      tokio::select! {
        biased;
        Some(expired) = PollExpired(&mut this.queue), if !this.queue.is_empty() => {
          this.on_expired(expired);
        }
        Some(signal) = recv.recv() => match signal {
          Signal::Init(signal) => this.on_init(signal),
          Signal::Read(signal) => this.on_read(signal),
          Signal::Stop(signal) => this.on_stop(signal),
          Signal::Quit => break 'work this.on_quit(recv),
        }
      }
    }
  }

  fn on_expired(&mut self, expired: Expired<TimerRef>) {
    let tref: TimerRef = expired.into_inner();

    if let Some(entry) = self.cache.remove(&tref) {
      tracing::trace!(id = self.id, %tref, "timer expired");
      send_timeout(entry.from, entry.dest, entry.term);
    } else {
      raise!(Error, SysInv, "dangling timer");
    }
  }

  fn on_init(&mut self, signal: InitTimer) {
    debug_assert_eq!(
      self.cache.len(),
      self.queue.len(),
      "wheel worker cache/queue desync",
    );

    tracing::trace!(id = self.id, tref = %signal.tref, from = %signal.from, "timer init");

    if self.cache.contains_key(&signal.tref) {
      raise!(Error, SysInv, "duplicate timer");
    }

    let data: TimerState = TimerState {
      from: signal.from,
      dest: signal.dest,
      term: signal.term,
      time: signal.ends,
      qkey: self.queue.insert_at(signal.tref, signal.ends.into()),
    };

    self.cache.insert(signal.tref, data);
  }

  fn on_read(&mut self, signal: ReadTimer) {
    let info: Option<Duration> = self.cache.get(&signal.tref).and_then(TimerState::remaining);

    tracing::trace!(id = self.id, tref = %signal.tref, "timer read");

    send_read_ack(signal, info);
  }

  fn on_stop(&mut self, signal: StopTimer) {
    let info: Option<Duration> = if let Some(data) = self.cache.remove(&signal.tref) {
      self.queue.remove(&data.qkey);

      tracing::trace!(id = self.id, tref = %signal.tref, "timer cancelled");

      data.remaining()
    } else {
      tracing::trace!(id = self.id, tref = %signal.tref, "timer not found");
      None
    };

    send_stop_ack(signal, info);
  }

  fn on_quit(&mut self, recv: UnboundedReceiver<Signal>) {
    tracing::info!(
      id = self.id,
      active = self.cache.len(),
      "wheel worker quitting"
    );

    for (tref, data) in self.cache.drain() {
      self.queue.remove(&data.qkey);
      tracing::trace!(id = self.id, tref = %tref, "timer cancelled");
    }

    Self::drain_channel(recv);

    tracing::info!(id = self.id, "wheel worker quit");
  }

  fn drain_channel(mut recv: UnboundedReceiver<Signal>) {
    recv.close();

    while let Ok(_) = recv.try_recv() {
      // ignore all remaining signals
    }
  }
}

fn send_timeout(from: InternalPid, to: InternalDest, term: Term) {
  match to {
    InternalDest::Proc(pid) => {
      if let Some(proc) = bifs::proc_find(pid) {
        proc.readonly.send_message(from, term);
        tracing::trace!(%from, to = %pid, "timeout sent");
      } else {
        tracing::trace!(%from, to = %pid, "dead PID");
      }
    }
    InternalDest::Name(name) => {
      if let Some(pid) = bifs::proc_whereis(name) {
        if let Some(proc) = bifs::proc_find(pid) {
          proc.readonly.send_message(from, term);
          tracing::trace!(%from, to = %pid, "timeout sent");
        } else {
          tracing::trace!(%from, to = %pid, "dead PID");
        }
      } else {
        tracing::trace!(%from, to = %name, "unregistered name");
      }
    }
  }
}

fn send_read_ack(signal: ReadTimer, info: Option<Duration>) {
  match signal.mode {
    ReplyMode::Oneshot(send) => {
      let _ignore: Result<(), Option<Duration>> = send.send(info);
    }
    ReplyMode::Mailbox(pid) => {
      if let Some(proc) = bifs::proc_find(pid) {
        let data: ReadTimerAck = ReadTimerAck {
          tref: signal.tref,
          info,
        };

        proc
          .readonly
          .send_message(InternalPid::ROOT_PROC, Term::new(data));
      }
    }
  }
}

fn send_stop_ack(signal: StopTimer, info: Option<Duration>) {
  match signal.mode {
    ReplyMode::Oneshot(send) => {
      let _ignore: Result<(), Option<Duration>> = send.send(info);
    }
    ReplyMode::Mailbox(pid) => {
      if let Some(proc) = bifs::proc_find(pid) {
        let data: StopTimerAck = StopTimerAck {
          tref: signal.tref,
          info,
        };

        proc
          .readonly
          .send_message(InternalPid::ROOT_PROC, Term::new(data));
      }
    }
  }
}
