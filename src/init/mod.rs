//! Errai runtime initialization and lifecycle management

/// Runs the given `future` to completion on the Errai runtime system.
///
/// This is the main entrypoint to the Errai runtime.
pub fn block_on<F>(future: F) -> !
where
  F: Future<Output = ()> + Send + 'static,
{
  todo!()
}
