macro_rules! decode_bitflags {
  ($type:ty) => {
    impl $crate::decode::Decode<'_> for $type {
      #[inline]
      fn decode(decoder: &mut $crate::decode::Decoder<'_>) -> $crate::error::Result<Self> {
        decoder.decode().map(<$type>::from_bits_retain)
      }
    }
  };
}

macro_rules! copy_into_owned {
  ($type:ty) => {
    impl $crate::traits::IntoOwned for $type {
      type Owned = Self;

      #[inline]
      fn into_owned(self) -> Self::Owned {
        self
      }
    }
  };
  ($($type:ty),+) => {
    $(
      copy_into_owned!($type);
    )+
  };
}

macro_rules! impl_content {
  (
    $(#[$meta:meta])*
    pub enum Content<'a> {
      $($(#[$vmeta:meta])* $variant:ident($inner:ty)),+
      $(,)?
    }
  ) => {
    $(#[$meta])*
    pub enum Content<'a> {
      $(
        $(#[$vmeta])*
        $variant($inner)
      ),+
    }

    impl ::core::fmt::Debug for Content<'_> {
      fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        match self {
          $(Self::$variant(inner) => ::core::fmt::Debug::fmt(inner, f)),+
        }
      }
    }

    impl $crate::traits::IntoOwned for Content<'_> {
      type Owned = Content<'static>;

      #[inline]
      fn into_owned(self) -> Self::Owned {
        match self {
          $(Self::$variant(inner) => Content::$variant(inner.into_owned())),+
        }
      }
    }
  };
}
