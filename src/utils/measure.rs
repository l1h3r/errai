//! Basic execution timing utilities.

use std::time::Duration;
use std::time::Instant;

#[inline(always)]
pub(crate) fn measure_fn<F>(f: F) -> Duration
where
  F: FnOnce(),
{
  let instant: Instant = Instant::now();

  f();

  instant.elapsed()
}
