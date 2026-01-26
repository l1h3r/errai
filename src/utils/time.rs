use std::time::Duration;
use std::time::SystemTime;

/// Returns the current OS system time as a POSIX duration.
///
/// This returns the duration since the Unix epoch (January 1, 1970 UTC).
/// If system time is before the epoch, returns [`Duration::ZERO`].
#[inline]
pub(crate) fn unix() -> Duration {
  SystemTime::now()
    .duration_since(SystemTime::UNIX_EPOCH)
    .unwrap_or(Duration::ZERO)
}
