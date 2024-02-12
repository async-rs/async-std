use core::future::Future;
use core::pin::Pin;

use crate::task::{Context, Poll};

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
pub fn ready<T>(val: T) -> Ready<T> {
    Ready(Some(val))
}

/// This future is constructed by the [`ready`] function.
///
/// [`ready`]: fn.ready.html
#[derive(Debug)]
pub struct Ready<T>(Option<T>);

impl<T> Unpin for Ready<T> {}

impl<T> Future for Ready<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<T> {
        Poll::Ready(self.0.take().unwrap())
    }
}
