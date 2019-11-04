use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use pin_project_lite::pin_project;

use crate::prelude::*;
use crate::stream::Fuse;

pin_project! {
    /// A stream that merges two other streams into a single stream.
    ///
    /// This `struct` is created by the [`merge`] method on [`Stream`]. See its
    /// documentation for more.
    ///
    /// [`merge`]: trait.Stream.html#method.merge
    /// [`Stream`]: trait.Stream.html
    #[cfg(feature = "unstable")]
    #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
    #[derive(Debug)]
    pub struct Merge<L, R> {
        #[pin]
        left: Fuse<L>,
        #[pin]
        right: Fuse<R>,
    }
}

impl<L: Stream, R: Stream> Merge<L, R> {
    pub(crate) fn new(left: L, right: R) -> Self {
        Self { left: left.fuse(), right: right.fuse() }
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
        match this.left.poll_next(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some(item)),
            Poll::Ready(None) => this.right.poll_next(cx),
            Poll::Pending => match this.right.poll_next(cx) {
                Poll::Ready(Some(item)) => Poll::Ready(Some(item)),
                Poll::Ready(None) => Poll::Pending,
                Poll::Pending => Poll::Pending,
            }
        }
    }
}
