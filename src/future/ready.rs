use std::future::Future;
use std::task::{Context, Poll};
use std::pin::Pin;

/// A future that immediately resolves.
///
/// This `struct` is created by the [`ready`] function. See its
/// documentation for more.
///
/// [`ready`]: fn.ready.html
#[derive(Debug, Clone)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct Ready<T>(Option<T>);

impl<T> Unpin for Ready<T> {}

impl<T> Future for Ready<T> {
    type Output = T;

    #[inline]
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<T> {
        Poll::Ready(self.0.take().unwrap())
    }
}

/// Resolves to the provided value.
///
/// This function is an async version of [`std::convert::identity`].
///
/// [`std::convert::identity`]: https://doc.rust-lang.org/std/convert/fn.identity.html
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use async_std::future;
///
/// assert_eq!(future::ready(10).await, 10);
/// #
/// # })
/// ```
pub fn ready<T>(t: T) -> Ready<T> {
    Ready(Some(t))
}
