use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

/// A stream that merges two other streams into a single stream.
///
/// This stream is returned by [`Stream::merge`].
///
/// [`Stream::merge`]: trait.Stream.html#method.merge
#[cfg(any(feature = "unstable", feature = "docs"))]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[derive(Debug)]
pub struct Merge<L, R> {
    left: L,
    right: R,
}

impl<L, R> Unpin for Merge<L, R> {}

impl<L, R> Merge<L, R> {
    pub(crate) fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L, R, T> Stream for Merge<L, R>
where
    L: Stream<Item = T> + Unpin,
    R: Stream<Item = T> + Unpin,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Poll::Ready(Some(item)) = Pin::new(&mut self.left).poll_next(cx) {
            // The first stream made progress. The Merge needs to be polled
            // again to check the progress of the second stream.
            cx.waker().wake_by_ref();
            Poll::Ready(Some(item))
        } else {
            Pin::new(&mut self.right).poll_next(cx)
        }
    }
}
