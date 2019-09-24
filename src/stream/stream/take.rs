use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream that yields the first `n` items of another stream.
#[derive(Clone, Debug)]
pub struct Take<S> {
    pub(crate) stream: S,
    pub(crate) remaining: usize,
}

impl<S: Unpin> Unpin for Take<S> {}

impl<S: Stream> Take<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(remaining: usize);
}

impl<S: Stream> Stream for Take<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        if self.remaining == 0 {
            Poll::Ready(None)
        } else {
            let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));
            match next {
                Some(_) => *self.as_mut().remaining() -= 1,
                None => *self.as_mut().remaining() = 0,
            }
            Poll::Ready(next)
        }
    }
}
