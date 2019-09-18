use std::error::Error;
use std::fmt;
use std::pin::Pin;
use std::time::Duration;

use futures_timer::Delay;

use crate::future::Future;
use crate::task::{Context, Poll};

/// Awaits a future or times out after a duration of time.
///
/// If you want to await an I/O future consider using
/// [`io::timeout`](../io/fn.timeout.html) instead.
///
/// # Examples
///
/// ```
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use std::time::Duration;
///
/// use async_std::future;
///
/// let never = future::pending::<()>();
/// let dur = Duration::from_millis(5);
/// assert!(future::timeout(dur, never).await.is_err());
/// #
/// # Ok(()) }) }
/// ```
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[cfg(feature = "unstable")]
pub async fn timeout<F, T>(dur: Duration, f: F) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    let f = TimeoutFuture {
        future: f,
        delay: Delay::new(dur),
    };
    f.await
}

/// A future that times out after a duration of time.
#[doc(hidden)]
#[allow(missing_debug_implementations)]
struct TimeoutFuture<F> {
    future: F,
    delay: Delay,
}

impl<F> TimeoutFuture<F> {
    pin_utils::unsafe_pinned!(future: F);
    pin_utils::unsafe_pinned!(delay: Delay);
}

impl<F: Future> Future for TimeoutFuture<F> {
    type Output = Result<F::Output, TimeoutError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().future().poll(cx) {
            Poll::Ready(v) => Poll::Ready(Ok(v)),
            Poll::Pending => match self.delay().poll(cx) {
                Poll::Ready(_) => Poll::Ready(Err(TimeoutError { _private: () })),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

/// An error returned when a future times out.
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[cfg(any(feature = "unstable", feature = "docs"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeoutError {
    _private: (),
}

impl Error for TimeoutError {}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "future has timed out".fmt(f)
    }
}
