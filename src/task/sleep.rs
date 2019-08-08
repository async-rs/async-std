use std::time::Duration;

use futures::prelude::*;

use crate::time::Timeout;

/// Sleeps for the specified amount of time.
///
/// This function might sleep for slightly longer than the specified duration but never less.
///
/// This function is an async version of [`std::thread::sleep`].
///
/// [`std::thread::sleep`]: https://doc.rust-lang.org/std/thread/fn.sleep.html
///
/// # Examples
///
/// ```
/// # #![feature(async_await)]
/// use async_std::task;
/// use std::time::Duration;
///
/// # async_std::task::block_on(async {
/// task::sleep(Duration::from_secs(1)).await;
/// # });
/// ```
pub async fn sleep(dur: Duration) {
    let _ = future::pending::<()>().timeout(dur).await;
}
