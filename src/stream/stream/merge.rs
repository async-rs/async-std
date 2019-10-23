use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use pin_project_lite::pin_project;

pin_project! {
    /// A stream that merges two other streams into a single stream.
    ///
    /// This stream is returned by [`Stream::merge`].
    ///
    /// [`Stream::merge`]: trait.Stream.html#method.merge
    #[cfg(feature = "unstable")]
    #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
    #[derive(Debug)]
    pub struct Merge<L, R> {
        #[pin]
        left: L,
        #[pin]
        right: R,
    }
}

impl<L, R> Merge<L, R> {
    pub(crate) fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L, R, T> Stream for Merge<L, R>
where
    L: Stream<Item = T>,
    R: Stream<Item = T>,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        if let Poll::Ready(Some(item)) = this.left.poll_next(cx) {
            // The first stream made progress. The Merge needs to be polled
            // again to check the progress of the second stream.
            cx.waker().wake_by_ref();
            Poll::Ready(Some(item))
        } else {
            this.right.poll_next(cx)
        }
    }
}
