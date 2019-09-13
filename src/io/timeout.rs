use std::time::Duration;

use futures_timer::TryFutureExt;

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
    f.timeout(dur).await
}
