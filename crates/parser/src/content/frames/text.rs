use alloc::borrow::Cow;
use core::fmt::Debug;
use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;

use crate::decode::Decode;
use crate::decode::Decoder;
use crate::decode::Encoding;
use crate::error::Result;
use crate::traits::IntoOwned;

// =============================================================================
// Text Information
// =============================================================================

/// Text information frame content.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Frame)]
pub struct Text<'a> {
  text_encoding: Encoding,
  #[frame(borrow)]
  text_content: TextContent<'a>,
}

// =============================================================================
// Text Content
// =============================================================================

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextContent<'a> {
  Text(Cow<'a, str>),
  List(Vec<Cow<'a, str>>),
}

impl<'a> Decode<'a> for TextContent<'a> {
  fn decode(decoder: &mut Decoder<'a>) -> Result<Self> {
    let text: Cow<'a, str> = decoder.decode()?;

    if decoder.is_empty() {
      return Ok(Self::Text(text));
    }

    let mut list: Vec<Cow<'a, str>> = vec![text];

    while !decoder.is_empty() {
      list.push(decoder.decode()?);
    }

    Ok(Self::List(list))
  }
}

impl IntoOwned for TextContent<'_> {
  type Owned = TextContent<'static>;

  #[inline]
  fn into_owned(self) -> Self::Owned {
    match self {
      Self::Text(inner) => TextContent::Text(IntoOwned::into_owned(inner)),
      Self::List(inner) => TextContent::List(IntoOwned::into_owned(inner)),
    }
  }
}

impl Debug for TextContent<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    match self {
      Self::Text(inner) => Debug::fmt(inner, f),
      Self::List(inner) => Debug::fmt(inner, f),
    }
  }
}

impl Display for TextContent<'_> {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    match self {
      Self::Text(inner) => {
        Display::fmt(inner, f)
      }
      Self::List(inner) => {
        let mut init: bool = false;

        for text in inner {
          if init {
            write!(f, ":")?;
          }

          write!(f, "{text}")?;
          init = true;
        }

        Ok(())
      }
    }
  }
}
