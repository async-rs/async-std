use std::pin::Pin;
use std::task::{Context, Poll};

/// A `Stream` that is permanently closed once a single call to `poll` results in
/// `Poll::Ready(None)`, returning `Poll::Ready(None)` for all future calls to `poll`.
#[derive(Clone, Debug)]
pub struct Fuse<S> {
    pub(crate) stream: S,
    pub(crate) done: bool,
}

impl<S: Unpin> Unpin for Fuse<S> {}

impl<S: futures_core::Stream> Fuse<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(done: bool);
}

impl<S: futures_core::Stream> futures_core::Stream for Fuse<S> {
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

impl<S: futures_core::Stream> futures_core::stream::FusedStream for Fuse<S> {
    fn is_terminated(&self) -> bool {
        self.done
    }
}
