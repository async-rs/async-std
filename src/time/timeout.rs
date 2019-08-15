use std::error::Error;
use std::fmt;
use std::pin::Pin;
use std::time::Duration;

use cfg_if::cfg_if;
use futures_timer::Delay;
use pin_utils::unsafe_pinned;

use crate::future::Future;
use crate::io;
use crate::task::{Context, Poll};

cfg_if! {
    if #[cfg(feature = "docs")] {
        #[doc(hidden)]
        pub struct ImplFuture<T>(std::marker::PhantomData<T>);

        macro_rules! ret {
            ($f:tt, $o:ty) => (ImplFuture<$o>);
        }
    } else {
        macro_rules! ret {
            ($f:tt, $o:ty) => ($f<Self>);
        }
    }
}

/// An error returned when a future times out.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeoutError;

impl Error for TimeoutError {}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "future has timed out".fmt(f)
    }
}

impl From<TimeoutError> for io::Error {
    fn from(_: TimeoutError) -> io::Error {
        io::Error::new(io::ErrorKind::TimedOut, "future has timed out")
    }
}

/// An extension trait that configures timeouts for futures.
pub trait Timeout: Future + Sized {
    /// Awaits a future to completion or times out after a duration of time.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use std::time::Duration;
    ///
    /// use async_std::io;
    /// use async_std::prelude::*;
    ///
    /// let stdin = io::stdin();
    /// let mut line = String::new();
    ///
    /// let n = stdin
    ///     .read_line(&mut line)
    ///     .timeout(Duration::from_secs(5))
    ///     .await??;
    /// #
    /// # Ok(()) }) }
    /// ```
    fn timeout(self, dur: Duration) -> ret!(TimeoutFuture, Result<Self::Output, TimeoutError>) {
        TimeoutFuture {
            future: self,
            delay: Delay::new(dur),
        }
    }
}

/// A future that times out after a duration of time.
#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct TimeoutFuture<F> {
    future: F,
    delay: Delay,
}

impl<F> TimeoutFuture<F> {
    unsafe_pinned!(future: F);
    unsafe_pinned!(delay: Delay);
}

impl<F: Future> Future for TimeoutFuture<F> {
    type Output = Result<F::Output, TimeoutError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().future().poll(cx) {
            Poll::Ready(v) => Poll::Ready(Ok(v)),
            Poll::Pending => match self.delay().poll(cx) {
                Poll::Ready(_) => Poll::Ready(Err(TimeoutError)),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

impl<F: Future> Timeout for F {}
