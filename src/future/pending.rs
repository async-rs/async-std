/// Never resolves to a value.
///
/// # Examples
///
/// ```
/// # fn main() { async_std::task::block_on(async {
/// #
/// use std::time::Duration;
///
/// use async_std::future;
/// use async_std::io;
///
/// let dur = Duration::from_secs(1);
/// let fut = future::pending();
///
/// let res: io::Result<()> = io::timeout(dur, fut).await;
/// assert!(res.is_err());
/// #
/// # }) }
/// ```
pub async fn pending<T>() -> T {
    futures::future::pending::<T>().await
}
