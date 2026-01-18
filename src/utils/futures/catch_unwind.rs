// Implementation borrowed from:
//
// https://docs.rs/futures/0.3.31/futures/future/struct.CatchUnwind.html

use pin_project_lite::pin_project;
use std::any::Any;
use std::panic;
use std::panic::AssertUnwindSafe;
use std::panic::UnwindSafe;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

pin_project! {
  #[derive(Debug)]
  #[repr(transparent)]
  pub(crate) struct CatchUnwind<F> {
    #[pin]
    future: F,
  }
}

impl<F> CatchUnwind<F>
where
  F: Future + UnwindSafe,
{
  #[inline]
  pub(crate) const fn new(future: F) -> Self {
    Self { future }
  }
}

impl<F> Future for CatchUnwind<F>
where
  F: Future + UnwindSafe,
{
  type Output = Result<F::Output, Box<dyn Any + Send>>;

  fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
    let future: Pin<&mut F> = self.project().future;
    let assert: AssertUnwindSafe<_> = AssertUnwindSafe(|| future.poll(context));

    panic::catch_unwind(assert)?.map(Ok)
  }
}
