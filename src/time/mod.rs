//! Timeouts for async operations.
//!
//! This module is an async extension of [`std::time`].
//!
//! [`std::time`]: https://doc.rust-lang.org/std/time/index.html
//!
//! # Examples
//!
//! Read a line from stdin with a timeout of 5 seconds.
//!
//! ```no_run
//! # #![feature(async_await)]
//! # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
//! #
//! use async_std::{io, prelude::*};
//! use std::time::Duration;
//!
//! let stdin = io::stdin();
//! let mut line = String::new();
//!
//! let n = stdin
//!     .read_line(&mut line)
//!     .timeout(Duration::from_secs(5))
//!     .await??;
//! #
//! # Ok(()) }) }
//! ```

use std::error::Error;
use std::fmt;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use cfg_if::cfg_if;
use futures_timer::Delay;
use pin_utils::unsafe_pinned;

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

cfg_if! {
    if #[cfg(feature = "docs.rs")] {
        #[doc(hidden)]
        pub struct ImplFuture<T>(std::marker::PhantomData<T>);

        /// An extension trait that configures timeouts for futures.
        pub trait Timeout: Future + Sized {
            /// Awaits a future to completion or times out after a duration of time.
            ///
            /// # Examples
            ///
            /// ```no_run
            /// # #![feature(async_await)]
            /// # fn main() -> io::Result<()> { async_std::task::block_on(async {
            /// #
            /// use async_std::{io, prelude::*};
            /// use std::time::Duration;
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
            fn timeout(self, dur: Duration) -> ImplFuture<Result<Self::Output, TimeoutError>> {
                TimeoutFuture {
                    future: self,
                    delay: Delay::new(dur),
                }
            }
        }
    } else {
        /// An extension trait that configures timeouts for futures.
        pub trait Timeout: Future + Sized {
            /// Awaits a future to completion or times out after a duration of time.
            fn timeout(self, dur: Duration) -> TimeoutFuture<Self> {
                TimeoutFuture {
                    future: self,
                    delay: Delay::new(dur),
                }
            }
        }

        /// A future that times out after a duration of time.
        #[doc(hidden)]
        #[derive(Debug)]
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
    }
}

impl<F: Future> Timeout for F {}
