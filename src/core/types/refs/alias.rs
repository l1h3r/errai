macro_rules! make_ref {
  (
    $(#[$meta:meta])+
    pub struct $name:ident;
  ) => {
    $(#[$meta])+
    #[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
    #[repr(transparent)]
    pub struct $name {
      inner: $crate::core::LocalRef,
    }

    impl $name {
      #[doc = concat!("Creates a new `", stringify!($name), "`.")]
      #[inline]
      pub(crate) fn new() -> Self {
        Self {
          inner: $crate::core::LocalRef::new(),
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
  pub struct AliasRef;
}

make_ref! {
  /// Reference uniquely identifying a process monitor.
  ///
  /// Monitor references are returned when establishing process monitors and
  /// are used to match down messages when the monitored process terminates.
  pub struct MonitorRef;
}

make_ref! {
  /// Reference uniquely identifying a timer.
  ///
  /// Timer references are returned when creating timers and can be used
  /// to cancel or query timer state.
  pub struct TimerRef;
}
