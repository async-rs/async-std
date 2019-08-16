use std::pin::Pin;

use crate::task::{Context, Poll};

/// A `Stream` that is permanently closed
/// once a single call to `poll` results in
/// `Poll::Ready(None)`, returning `Poll::Ready(None)`
/// for all future calls to `poll`.
#[derive(Clone, Debug)]
pub struct Fuse<S> {
    stream: S,
    done: bool,
}

impl<S: Unpin> Unpin for Fuse<S> {}

impl<S: futures::Stream> Fuse<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(done: bool);

    /// Returns `true` if the underlying stream is fused.
    ///
    /// If this `Stream` is fused, all future calls to
    /// `poll` will return `Poll::Ready(None)`.
    pub fn is_done(&self) -> bool {
        self.done
    }

    /// Consumes this `Fuse` and returns the inner
    /// `Stream`, unfusing it if it had become
    /// fused.
    pub fn into_inner(self) -> S
    where
        S: Sized,
    {
        self.stream
    }
}


impl<S: futures::Stream> futures::Stream for Fuse<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        if self.done {
            Poll::Ready(None)
        } else {
            let next = futures::ready!(self.as_mut().stream().poll_next(cx));
            if next.is_none() {
                *self.as_mut().done() = true;
            }
            Poll::Ready(next)
        }
    }
}
