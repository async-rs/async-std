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
