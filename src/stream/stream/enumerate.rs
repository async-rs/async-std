use crate::task::{Context, Poll};
use std::pin::Pin;

use crate::stream::Stream;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Enumerate<S> {
    stream: S,
    i: usize,
}

impl<S> Enumerate<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(i: usize);

    pub(super) fn new(stream: S) -> Self {
        Enumerate { stream, i: 0 }
    }
}

impl<S> Stream for Enumerate<S>
where
    S: Stream,
{
    type Item = (usize, S::Item);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match next {
            Some(v) => {
                let ret = (self.i, v);
                *self.as_mut().i() += 1;
                Poll::Ready(Some(ret))
            }
            None => Poll::Ready(None),
        }
    }
}
