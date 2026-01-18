//! Base message type for process communication.

use std::any::TypeId;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use crate::core::Term;
use crate::erts::DownMessage;
use crate::erts::ExitMessage;

/// Type alias for messages containing dynamic terms.
pub type DynMessage = Message<Term>;

/// A message received by a process.
///
/// Messages come in three varieties:
///
/// 1. **Term**: Regular user-sent message containing arbitrary data
/// 2. **Exit**: Trapped exit signal from a linked process
/// 3. **Down**: Monitor notification for a watched process
///
/// # Selective Receive
///
/// The [`is()`] and [`is_exact()`] methods enable filtering messages by
/// type during selective receive operations.
///
/// # Downcasting
///
/// Messages containing [`Term`] can be downcast to concrete types using
/// unsafe downcasting methods after type checking.
///
/// [`is()`]: Self::is
/// [`is_exact()`]: Self::is_exact
#[derive(Clone)]
pub enum Message<T = Term> {
  /// Regular user message containing arbitrary data.
  Term(T),
  /// Trapped exit signal from a linked process.
  Exit(ExitMessage),
  /// Monitor down notification for a watched process.
  Down(DownMessage),
}

impl<T> Message<T> {
  /// Returns `true` if the message is a term message.
  #[inline]
  pub fn is_term(&self) -> bool {
    matches!(self, Self::Term(_))
  }

  /// Returns `true` if the message is a trapped EXIT signal.
  ///
  /// EXIT signals only appear as messages when the receiving process
  /// has the `trap_exit` flag enabled.
  #[inline]
  pub fn is_exit(&self) -> bool {
    matches!(self, Self::Exit(_))
  }

  /// Returns `true` if the message is a DOWN signal.
  ///
  /// DOWN signals are sent when a monitored process terminates.
  #[inline]
  pub fn is_down(&self) -> bool {
    matches!(self, Self::Down(_))
  }
}

impl Message<Term> {
  /// Returns `true` if the message matches type `T` or is EXIT/DOWN.
  ///
  /// This method is used in selective receive to match messages:
  ///
  /// - For term messages: checks if the term contains type `T`
  /// - For EXIT/DOWN: always returns `true`
  ///
  /// Use [`is_exact()`] if you need to distinguish EXIT/DOWN from terms.
  ///
  /// [`is_exact()`]: Self::is_exact
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

  /// Returns `true` if the message exactly matches type `T`.
  ///
  /// This method performs strict type checking:
  ///
  /// - For term messages: checks if the term contains type `T`
  /// - For EXIT: checks if `T` is [`ExitMessage`]
  /// - For DOWN: checks if `T` is [`DownMessage`]
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

  /// Downcasts the term message to a concrete type.
  ///
  /// EXIT and DOWN messages are preserved as-is.
  ///
  /// # Safety
  ///
  /// If this is a term message, the contained value **must** be of type `T`.
  /// Use [`is()`] to verify the type before calling.
  ///
  /// [`is()`]: Self::is
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

  /// Downcasts the message to a boxed concrete type.
  ///
  /// This method handles all three message variants:
  ///
  /// - Term: downcasts the term to `Box<T>`
  /// - Exit: casts [`ExitMessage`] to `Box<T>`
  /// - Down: casts [`DownMessage`] to `Box<T>`
  ///
  /// # Safety
  ///
  /// The message **must** match type `T`:
  ///
  /// - For terms: the term must contain `T`
  /// - For EXIT: `T` must be [`ExitMessage`]
  /// - For DOWN: `T` must be [`DownMessage`]
  ///
  /// Use [`is_exact()`] to verify the type before calling.
  ///
  /// [`is_exact()`]: Self::is_exact
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
// Misc. Utilities
// -----------------------------------------------------------------------------

/// Returns `true` if `T` is [`ExitMessage`].
#[inline]
fn is_exit_type<T>() -> bool
where
  T: 'static,
{
  TypeId::of::<T>() == TypeId::of::<ExitMessage>()
}

/// Returns `true` if `T` is [`DownMessage`].
#[inline]
fn is_down_type<T>() -> bool
where
  T: 'static,
{
  TypeId::of::<T>() == TypeId::of::<DownMessage>()
}

/// Downcasts a term to the concrete type `T`.
///
/// # Safety
///
/// The term must contain type `T`.
#[inline]
unsafe fn downcast_term_unchecked<T>(term: Term) -> Box<T>
where
  T: 'static,
{
  unsafe { term.downcast_unchecked() }
}

/// Casts an exit message to `Box<T>`.
///
/// # Safety
///
/// `T` must be [`ExitMessage`].
#[inline]
unsafe fn downcast_exit_unchecked<T>(exit: ExitMessage) -> Box<T>
where
  T: 'static,
{
  // SAFETY: This is guaranteed to be safe by the caller.
  unsafe { Box::from_raw(Box::into_raw(Box::new(exit)).cast::<T>()) }
}

/// Casts a down message to `Box<T>`.
///
/// # Safety
///
/// `T` must be [`DownMessage`].
#[inline]
unsafe fn downcast_down_unchecked<T>(down: DownMessage) -> Box<T>
where
  T: 'static,
{
  // SAFETY: This is guaranteed to be safe by the caller.
  unsafe { Box::from_raw(Box::into_raw(Box::new(down)).cast::<T>()) }
}
