macro_rules! impl_stack_string {
  (
    #[doc = $doc:literal]
    @ident = $ident:ident;
    @bytes = $bytes:expr;
    @check = $check:ident;
  ) => {
    #[doc = $doc]
    #[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
    pub struct $ident {
      inner: [u8; $bytes],
    }

    impl $ident {
      /// Get a string representation of `self`.
      #[inline]
      pub const fn as_str(&self) -> &str {
        // SAFETY: We validated the input bytes on creation
        unsafe { ::core::str::from_utf8_unchecked(self.as_slice()) }
      }

      /// Get a shared reference to the underlying array of bytes.
      #[inline]
      pub const fn as_array(&self) -> &[u8; $bytes] {
        &self.inner
      }

      /// Get a shared reference to the underlying slice of bytes.
      #[inline]
      pub const fn as_slice(&self) -> &[u8] {
        &self.inner
      }
    }

    impl ::core::fmt::Debug for $ident {
      fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        ::core::fmt::Debug::fmt(self.as_str(), f)
      }
    }

    impl ::core::fmt::Display for $ident {
      fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        ::core::fmt::Display::fmt(self.as_str(), f)
      }
    }

    impl ::core::ops::Deref for $ident {
      type Target = str;

      #[inline]
      fn deref(&self) -> &Self::Target {
        self.as_str()
      }
    }

    impl $crate::decode::Decode<'_> for $ident {
      fn decode(decoder: &mut $crate::decode::Decoder<'_>) -> $crate::error::Result<Self> {
        let inner: [u8; $bytes] = decoder.decode()?;

        if $check(&inner) {
          Ok(Self { inner })
        } else {
          Err($crate::error::Error::new(
            $crate::error::ErrorKind::InvalidFrameData,
          ))
        }
      }
    }

    copy_into_owned!($ident);
  };
}
