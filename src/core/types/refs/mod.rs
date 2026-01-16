mod external;
mod internal;

pub use self::external::ExternalRef;
pub use self::internal::InternalRef;

// -----------------------------------------------------------------------------
// Alias/Monitor/Timer References
// -----------------------------------------------------------------------------

macro_rules! make_ref {
  ($name:ident) => {
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

make_ref!(AliasRef);
make_ref!(MonitorRef);
make_ref!(TimerRef);
