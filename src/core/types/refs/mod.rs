//! Reference types used for monitoring, aliasing, and timers.
//!
//! This module provides unique identifiers for runtime objects and operations.
//! References are globally unique within a node and can be extended with node
//! information for distributed systems.
//!
//! # Reference Types
//!
//! - [`InternalRef`]: Base reference type for local node objects
//! - [`ExternalRef`]: Distributed reference with node identifier
//! - [`MonitorRef`]: Monitor-specific reference
//! - [`AliasRef`]: Process alias reference
//! - [`TimerRef`]: Timer identifier reference
//!
//! # Uniqueness Guarantees
//!
//! All references are generated from a monotonic global counter, ensuring
//! uniqueness within the node's lifetime. References are 96 bits (3xu32)
//! and incorporate timestamp-based initialization for cross-restart uniqueness.

mod external;
mod internal;

pub use self::external::ExternalRef;
pub use self::internal::InternalRef;

// -----------------------------------------------------------------------------
// Alias/Monitor/Timer References
// -----------------------------------------------------------------------------

macro_rules! make_ref {
  (
    $(#[$meta:meta])+
    struct $name:ident;
  ) => {
    $(#[$meta])+
    #[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
    #[repr(transparent)]
    pub struct $name {
      inner: InternalRef,
    }

    impl $name {
      #[doc = concat!("Creates a new `", stringify!($name), "`.")]
      pub(crate) fn new() -> Self {
        Self {
          inner: InternalRef::new_global(),
        }
      }
    }

    impl ::core::fmt::Debug for $name {
      fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        ::core::fmt::Debug::fmt(&self.inner, f)
      }
    }

    impl ::core::fmt::Display for $name {
      fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        ::core::fmt::Display::fmt(&self.inner, f)
      }
    }
  };
}

make_ref! {
  /// Reference uniquely identifying a process alias.
  ///
  /// Alias references provide temporary names for processes, enabling
  /// selective message reception and request-response patterns.
  struct AliasRef;
}

make_ref! {
  /// Reference uniquely identifying a process monitor.
  ///
  /// Monitor references are returned when establishing process monitors and
  /// are used to match down messages when the monitored process terminates.
  struct MonitorRef;
}

make_ref! {
  /// Reference uniquely identifying a timer.
  ///
  /// Timer references are returned when creating timers and can be used
  /// to cancel or query timer state.
  struct TimerRef;
}
