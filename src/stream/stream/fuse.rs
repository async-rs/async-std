use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A `Stream` that is permanently closed once a single call to `poll` results in
/// `Poll::Ready(None)`, returning `Poll::Ready(None)` for all future calls to `poll`.
#[derive(Clone, Debug)]
pub struct Fuse<S> {
    pub(crate) stream: S,
    pub(crate) done: bool,
}

impl<S: Unpin> Unpin for Fuse<S> {}

impl<S: Stream> Fuse<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(done: bool);
}

impl<S: Stream> Stream for Fuse<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        if self.done {
            Poll::Ready(None)
        } else {
            let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));
            if next.is_none() {
                *self.as_mut().done() = true;
            }
            Poll::Ready(next)
        }
    }
}
