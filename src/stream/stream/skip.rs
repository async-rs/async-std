use std::pin::Pin;
use std::task::{Context, Poll};

use crate::stream::Stream;

/// A stream to skip first n elements of another stream.
#[derive(Debug)]
pub struct Skip<S> {
    stream: S,
    n: usize,
}

impl<S> Skip<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(n: usize);

    pub(crate) fn new(stream: S, n: usize) -> Self {
        Skip { stream, n }
    }
}

impl<S> Stream for Skip<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

            match next {
                Some(v) => match self.n {
                    0 => return Poll::Ready(Some(v)),
                    _ => *self.as_mut().n() -= 1,
                },
                None => return Poll::Ready(None),
            }
        }
    }
}
