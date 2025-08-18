use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

cfg_unstable! {
    use crate::stream::{DoubleEndedStream, ExactSizeStream, FusedStream};
}

use crate::stream::Stream;

/// A stream that never returns any items.
///
/// This stream is created by the [`pending`] function. See its
/// documentation for more.
///
/// [`pending`]: fn.pending.html
#[derive(Debug)]
pub struct Pending<T> {
    _marker: PhantomData<T>,
}

/// Creates a stream that never returns any items.
///
/// The returned stream will always return `Pending` when polled.
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use std::time::Duration;
///
/// use async_std::prelude::*;
/// use async_std::stream;
///
/// let dur = Duration::from_millis(100);
/// let mut s = stream::pending::<()>().timeout(dur);
///
/// let item = s.next().await;
///
/// assert!(item.is_some());
/// assert!(item.unwrap().is_err());
///
/// #
/// # })
/// ```
pub fn pending<T>() -> Pending<T> {
    Pending {
        _marker: PhantomData,
    }
}

impl<T> Stream for Pending<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<T>> {
        Poll::Pending
    }
}

#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
impl<T> DoubleEndedStream for Pending<T> {
    fn poll_next_back(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<T>> {
        Poll::Pending
    }
}

#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
impl<T> FusedStream for Pending<T> {}

#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
impl<T> ExactSizeStream for Pending<T> {
    fn len(&self) -> usize {
        0
    }
}
