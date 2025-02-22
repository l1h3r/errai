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
