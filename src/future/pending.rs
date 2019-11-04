use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::task::{Context, Poll};

/// A future that never resolves.
///
/// This `struct` is created by the [`pending`] function. See its
/// documentation for more.
///
/// [`pending`]: fn.pending.html
#[derive(Debug, Clone)]
pub struct Pending<T> {
    _marker: PhantomData<T>,
}

/// Never resolves to a value.
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
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
/// # })
/// ```
pub async fn pending<T>() -> T {
    let fut = Pending {
        _marker: PhantomData,
    };
    fut.await
}

impl<T> Future for Pending<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<T> {
        Poll::Pending
    }
}
