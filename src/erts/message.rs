use std::any::TypeId;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use crate::lang::Exit;
use crate::lang::ExternalDest;
use crate::lang::InternalPid;
use crate::lang::MonitorRef;
use crate::lang::Term;

pub type DynMessage = Message<Term>;

// -----------------------------------------------------------------------------
// Message
// -----------------------------------------------------------------------------

/// A converted process message.
#[derive(Clone)]
pub enum Message<T = Term> {
  Term(T),
  Exit(ExitMessage),
  Down(DownMessage),
}

impl<T> Message<T> {
  /// Returns `true` if the message is a term.
  #[inline]
  pub fn is_term(&self) -> bool {
    matches!(self, Self::Term(_))
  }

  /// Returns `true` if the message is a trapped EXIT signal.
  #[inline]
  pub fn is_exit(&self) -> bool {
    matches!(self, Self::Exit(_))
  }

  /// Returns `true` if the message is a DOWN signal.
  #[inline]
  pub fn is_down(&self) -> bool {
    matches!(self, Self::Down(_))
  }
}

impl Message<Term> {
  /// Returns `true` if the message value matches `T`,
  /// or the message is a trapped EXIT signal.
  #[inline]
  pub fn is<T>(&self) -> bool
  where
    T: 'static,
  {
    match self {
      Self::Term(inner) => inner.is::<T>(),
      Self::Exit(_) => true,
      Self::Down(_) => true,
    }
  }

  /// Returns `true` if the message value matches `T`.
  #[inline]
  pub fn is_exact<T>(&self) -> bool
  where
    T: 'static,
  {
    match self {
      Self::Term(inner) => inner.is::<T>(),
      Self::Exit(_) => is_exit_type::<T>(),
      Self::Down(_) => is_down_type::<T>(),
    }
  }

  /// Downcasts the boxed term to a concrete type.
  ///
  /// # Safety
  ///
  /// The contained value must be of type `T`. Calling this method with the
  /// incorrect type is undefined behavior.
  #[inline]
  pub unsafe fn downcast_unchecked<T>(self) -> Message<Box<T>>
  where
    T: 'static,
  {
    match self {
      // SAFETY: This is guaranteed to be safe by the caller.
      Self::Term(inner) => Message::Term(unsafe { downcast_term_unchecked(inner) }),
      Self::Exit(inner) => Message::Exit(inner),
      Self::Down(inner) => Message::Down(inner),
    }
  }

  /// Downcasts the boxed term to a concrete type.
  ///
  /// # Safety
  ///
  /// The contained value must be of type `T`. Calling this method with the
  /// incorrect type is undefined behavior.
  #[inline]
  pub unsafe fn downcast_exact_unchecked<T>(self) -> Box<T>
  where
    T: 'static,
  {
    match self {
      // SAFETY: This is guaranteed to be safe by the caller.
      Self::Term(inner) => unsafe { downcast_term_unchecked(inner) },
      // SAFETY: This is guaranteed to be safe by the caller.
      Self::Exit(inner) => unsafe { downcast_exit_unchecked(inner) },
      // SAFETY: This is guaranteed to be safe by the caller.
      Self::Down(inner) => unsafe { downcast_down_unchecked(inner) },
    }
  }
}

impl<T> Debug for Message<T>
where
  T: Debug,
{
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    match self {
      Self::Term(inner) => Debug::fmt(inner, f),
      Self::Exit(inner) => Debug::fmt(inner, f),
      Self::Down(inner) => Debug::fmt(inner, f),
    }
  }
}

impl From<Term> for Message<Term> {
  #[inline]
  fn from(other: Term) -> Self {
    Self::Term(other)
  }
}

impl<T> From<ExitMessage> for Message<T> {
  #[inline]
  fn from(other: ExitMessage) -> Self {
    Self::Exit(other)
  }
}

impl<T> From<DownMessage> for Message<T> {
  #[inline]
  fn from(other: DownMessage) -> Self {
    Self::Down(other)
  }
}

impl<T> TryFrom<Message<Term>> for Message<Box<T>>
where
  T: 'static,
{
  type Error = Message<Term>;

  #[inline]
  fn try_from(other: Message<Term>) -> Result<Self, Self::Error> {
    if other.is::<T>() {
      // SAFETY: We just ensured the source message contains a valid `T`.
      Ok(unsafe { other.downcast_unchecked::<T>() })
    } else {
      Err(other)
    }
  }
}

// -----------------------------------------------------------------------------
// Exit Message
// -----------------------------------------------------------------------------

/// A message representing a trapped EXIT signal.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ExitMessage {
  from: InternalPid,
  exit: Exit,
}

impl ExitMessage {
  /// Creates a new `ExitMessage`.
  #[inline]
  pub(crate) fn new(from: InternalPid, exit: Exit) -> Self {
    Self {
      from: from.into(),
      exit,
    }
  }

  /// Returns a reference to the EXIT signal sender.
  #[inline]
  pub const fn from(&self) -> InternalPid {
    self.from
  }

  /// Returns the EXIT signal exit reason.
  #[inline]
  pub const fn exit(&self) -> &Exit {
    &self.exit
  }
}

// -----------------------------------------------------------------------------
// Down Message
// -----------------------------------------------------------------------------

/// A message representing a monitor DOWN signal.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct DownMessage {
  mref: MonitorRef,
  item: ExternalDest,
  info: Exit,
}

impl DownMessage {
  /// Creates a new `DownMessage`.
  #[inline]
  pub(crate) fn new(mref: MonitorRef, item: ExternalDest, info: Exit) -> Self {
    Self { mref, item, info }
  }

  /// Returns a reference to the monitored reference.
  #[inline]
  pub const fn mref(&self) -> &MonitorRef {
    &self.mref
  }

  /// Returns a reference to the monitored item.
  #[inline]
  pub const fn item(&self) -> &ExternalDest {
    &self.item
  }

  /// Returns the DOWN signal exit reason.
  #[inline]
  pub const fn info(&self) -> &Exit {
    &self.info
  }
}

// -----------------------------------------------------------------------------
// Misc. Utilities
// -----------------------------------------------------------------------------

#[inline]
fn is_exit_type<T>() -> bool
where
  T: 'static,
{
  TypeId::of::<T>() == TypeId::of::<ExitMessage>()
}

#[inline]
fn is_down_type<T>() -> bool
where
  T: 'static,
{
  TypeId::of::<T>() == TypeId::of::<DownMessage>()
}

#[inline]
unsafe fn downcast_term_unchecked<T>(term: Term) -> Box<T>
where
  T: 'static,
{
  unsafe { term.downcast_unchecked() }
}

#[inline]
unsafe fn downcast_exit_unchecked<T>(exit: ExitMessage) -> Box<T>
where
  T: 'static,
{
  // SAFETY: This is guaranteed to be safe by the caller.
  unsafe { Box::from_raw(Box::into_raw(Box::new(exit)).cast::<T>()) }
}

#[inline]
unsafe fn downcast_down_unchecked<T>(down: DownMessage) -> Box<T>
where
  T: 'static,
{
  // SAFETY: This is guaranteed to be safe by the caller.
  unsafe { Box::from_raw(Box::into_raw(Box::new(down)).cast::<T>()) }
}
