use crate::task::{Context, Poll};

/// Creates a new future wrapping around a function returning [`Poll`].
///
/// Polling the returned future delegates to the wrapped function.
///
/// # Examples
///
/// ```
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::future;
/// use async_std::task::{Context, Poll};
///
/// fn poll_greeting(_: &mut Context<'_>) -> Poll<String> {
///     Poll::Ready("hello world".to_string())
/// }
///
/// assert_eq!(future::poll_fn(poll_greeting).await, "hello world");
/// #
/// # }) }
/// ```
pub async fn poll_fn<F, T>(f: F) -> T
where
    F: FnMut(&mut Context<'_>) -> Poll<T>,
{
    futures::future::poll_fn(f).await
}
