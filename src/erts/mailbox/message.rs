use std::any::TypeId;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use crate::lang::DynPid;
use crate::lang::ExitReason;
use crate::lang::Term;

// -----------------------------------------------------------------------------
// Message
// -----------------------------------------------------------------------------

/// A strongly-typed process message.
#[derive(Clone)]
pub enum Message<T> {
  Term(T),
  Exit(ExitMessage),
}

impl<T> Message<T> {
  /// Returns `true` if the message is a term.
  #[inline]
  pub fn is_term(&self) -> bool {
    matches!(self, Self::Term(_))
  }

  /// Returns `true` if the message is a trapped exit signal.
  #[inline]
  pub fn is_exit(&self) -> bool {
    matches!(self, Self::Exit(_))
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
    }
  }
}

impl<T> TryFrom<DynMessage> for Message<Box<T>>
where
  T: 'static,
{
  type Error = DynMessage;

  #[inline]
  fn try_from(other: DynMessage) -> Result<Self, Self::Error> {
    if other.is::<T>() {
      // SAFETY: We just ensured the source message contains a valid `T`.
      Ok(unsafe { other.downcast_unchecked::<T>() })
    } else {
      Err(other)
    }
  }
}

// -----------------------------------------------------------------------------
// Dyn Message
// -----------------------------------------------------------------------------

/// A dynamically-typed process message.
#[derive(Clone)]
pub enum DynMessage {
  Term(Term),
  Exit(ExitMessage),
}

impl DynMessage {
  /// Returns `true` if the message is a term.
  #[inline]
  pub fn is_term(&self) -> bool {
    matches!(self, Self::Term(_))
  }

  /// Returns `true` if the message is a trapped exit signal.
  #[inline]
  pub fn is_exit(&self) -> bool {
    matches!(self, Self::Exit(_))
  }

  /// Returns `true` if the message value matches `T`,
  /// or the message is a trapped exit signal.
  #[inline]
  pub fn is<T>(&self) -> bool
  where
    T: 'static,
  {
    match self {
      Self::Term(inner) => inner.is::<T>(),
      Self::Exit(_) => true,
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
    }
  }
}

impl Debug for DynMessage {
  fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
    match self {
      Self::Term(inner) => Debug::fmt(inner, f),
      Self::Exit(inner) => Debug::fmt(inner, f),
    }
  }
}

impl From<Term> for DynMessage {
  #[inline]
  fn from(other: Term) -> Self {
    Self::Term(other)
  }
}

impl From<ExitMessage> for DynMessage {
  #[inline]
  fn from(other: ExitMessage) -> Self {
    Self::Exit(other)
  }
}

// -----------------------------------------------------------------------------
// Exit Message
// -----------------------------------------------------------------------------

/// A message representing a trapped exit signal.
#[derive(Clone, Debug)]
#[repr(C)]
pub struct ExitMessage {
  sender: DynPid,
  reason: ExitReason,
}

impl ExitMessage {
  /// Creates a new `ExitMessage`.
  #[inline]
  pub(crate) fn new<T>(sender: T, reason: ExitReason) -> Self
  where
    T: Into<DynPid>,
  {
    Self {
      sender: sender.into(),
      reason,
    }
  }

  /// Returns a reference to the exit signal sender.
  #[inline]
  pub const fn sender(&self) -> &DynPid {
    &self.sender
  }

  /// Returns the exit signal reason.
  #[inline]
  pub const fn reason(&self) -> &ExitReason {
    &self.reason
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
