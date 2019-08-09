//! Asynchronous values.

#[doc(inline)]
pub use std::future::Future;

/// Never resolves to a value.
///
/// # Examples
/// ```
/// # #![feature(async_await)]
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::future::pending;
/// use async_std::prelude::*;
/// use std::time::Duration;
///
/// let dur = Duration::from_secs(1);
/// assert!(pending::<()>().timeout(dur).await.is_err());
/// #
/// # }) }
/// ```
pub async fn pending<T>() -> T {
    futures::future::pending::<T>().await
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
/// # #![feature(async_await)]
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::future::ready;
///
/// assert_eq!(ready(10).await, 10);
/// #
/// # }) }
/// ```
pub async fn ready<T>(val: T) -> T {
    val
}
