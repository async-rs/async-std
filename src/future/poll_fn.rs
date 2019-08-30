//! Definition of the `PollFn` adapter combinator

use core::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll};

/// Future for the [`poll_fn`] function.
#[must_use = "futures do nothing unless you `.await` or poll them"]
struct PollFn<F> {
    f: F,
}

impl<F> Unpin for PollFn<F> {}

/// Creates a new future wrapping around a function returning `Poll`.
///
/// Polling the returned future delegates to the wrapped function.
///
/// # Examples
///
/// ```
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::future::poll_fn;
/// use async_std::task::{Context, Poll};
///
/// fn read_line(_cx: &mut Context<'_>) -> Poll<String> {
///     Poll::Ready("Hello, World!".into())
/// }
///
/// let read_future = poll_fn(read_line);
/// assert_eq!(read_future.await, "Hello, World!");
/// #
/// # }) }
/// ```
pub async fn poll_fn<T>(f: impl FnMut(&mut Context<'_>) -> Poll<T>) -> T {
    let fut = PollFn { f };
    fut.await
}

impl<T, F> Future for PollFn<F>
where
    F: FnMut(&mut Context<'_>) -> Poll<T>,
{
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        (&mut self.f)(cx)
    }
}
