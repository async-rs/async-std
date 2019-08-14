use std::pin::Pin;
use std::time::Duration;

use futures_timer::Delay;
use pin_utils::unsafe_pinned;

use crate::future::Future;
use crate::io;
use crate::task::{Context, Poll};

/// Awaits an I/O future or times out after a duration of time.
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
///
/// let stdin = io::stdin();
/// let mut line = String::new();
///
/// let dur = Duration::from_secs(5);
/// let n = io::timeout(dur, stdin.read_line(&mut line)).await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn timeout<F, T>(dur: Duration, f: F) -> io::Result<T>
where
    F: Future<Output = io::Result<T>>,
{
    let f = TimeoutFuture {
        future: f,
        delay: Delay::new(dur),
    };
    f.await
}

struct TimeoutFuture<F> {
    future: F,
    delay: Delay,
}

impl<F> TimeoutFuture<F> {
    unsafe_pinned!(future: F);
    unsafe_pinned!(delay: Delay);
}

impl<F, T> Future for TimeoutFuture<F>
where
    F: Future<Output = io::Result<T>>,
{
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().future().poll(cx) {
            Poll::Ready(v) => Poll::Ready(v),
            Poll::Pending => match self.delay().poll(cx) {
                Poll::Ready(_) => Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "future has timed out",
                ))),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}
