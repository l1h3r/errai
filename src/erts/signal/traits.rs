use tracing::Span;

use crate::core::Exit;
use crate::erts::ProcInternal;
use crate::erts::ProcReadOnly;

// -----------------------------------------------------------------------------
// Signal Emit
// -----------------------------------------------------------------------------

/// Trait for sending signals to a process.
///
/// Implemented by all signal types to enable polymorphic signal sending.
/// Signals are enqueued in the target process's signal queue.
pub(crate) trait SignalEmit {
  /// Sends this signal to the target process.
  ///
  /// The signal is enqueued in the process's signal queue and will be
  /// processed asynchronously by the process task loop.
  fn emit(self, to: &ProcReadOnly);
}

// -----------------------------------------------------------------------------
// Signal Recv
// -----------------------------------------------------------------------------

/// Trait for processing received signals.
///
/// Implemented by all signal types to define their handling logic.
/// Signal processing may modify process state or trigger termination.
pub(crate) trait SignalRecv {
  /// Processes this signal in the context of the receiving process.
  ///
  /// Returns [`Exit`] if the signal should terminate the process,
  /// or [`None`] if processing completes without termination.
  ///
  /// # State Modifications
  ///
  /// Signal processing may:
  /// - Add/remove links or monitors
  /// - Enqueue messages in the inbox
  /// - Modify process flags
  fn recv(self, span: &Span, readonly: &ProcReadOnly, internal: &mut ProcInternal) -> Option<Exit>;
}
