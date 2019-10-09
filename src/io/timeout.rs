use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_timer::Delay;
use pin_utils::unsafe_pinned;

use crate::future::Future;
use crate::io;

/// Awaits an I/O future or times out after a duration of time.
///
/// If you want to await a non I/O future consider using
/// [`future::timeout`](../future/fn.timeout.html) instead.
///
/// # Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use std::time::Duration;
///
/// use async_std::io;
///
/// io::timeout(Duration::from_secs(5), async {
///     let stdin = io::stdin();
///     let mut line = String::new();
///     let n = stdin.read_line(&mut line).await?;
///     Ok(())
/// })
/// .await?;
/// #
/// # Ok(()) }) }
/// ```
pub async fn timeout<F, T>(dur: Duration, f: F) -> io::Result<T>
where
    F: Future<Output = io::Result<T>>,
{
    Timeout {
        timeout: Delay::new(dur),
        future: f,
    }
    .await
}

/// Future returned by the `FutureExt::timeout` method.
#[derive(Debug)]
pub struct Timeout<F, T>
where
    F: Future<Output = io::Result<T>>,
{
    future: F,
    timeout: Delay,
}

impl<F, T> Timeout<F, T>
where
    F: Future<Output = io::Result<T>>,
{
    unsafe_pinned!(future: F);
    unsafe_pinned!(timeout: Delay);
}

impl<F, T> Future for Timeout<F, T>
where
    F: Future<Output = io::Result<T>>,
{
    type Output = io::Result<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().future().poll(cx) {
            Poll::Pending => {}
            other => return other,
        }

        if self.timeout().poll(cx).is_ready() {
            let err = Err(io::Error::new(io::ErrorKind::TimedOut, "future timed out").into());
            Poll::Ready(err)
        } else {
            Poll::Pending
        }
    }
}
