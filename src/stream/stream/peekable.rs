use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};
use crate::stream::stream::StreamExt;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Peekable<S: Stream>
where
    S: Sized,
{
    stream: S,
    peeked: Option<Poll<Option<S::Item>>>,
}

impl<S> Peekable<S>
where
    S: Stream,
{
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(peeked: Option<Poll<Option<S::Item>>>);

    pub(crate) fn new(stream: S) -> Self {
        Peekable {
            stream: stream,
            peeked: None,
        }
    }

    pub fn peek(mut self: Pin<&mut Self>) -> &Poll<Option<S::Item>> {
        match &self.peeked {
            Some(peeked) => &peeked,
            None => {
                // how to get the next `next` value? What about `Context`
                let next = self.stream.next();
                *self.as_mut().peeked() = Some(next);
                &next
            }
        }
    }
}

impl<S> Stream for Peekable<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match &self.peeked {
            Some(peeked) => {
                let v = self.as_mut().peeked().take().unwrap();
                *self.as_mut().peeked() = None;

                v
            }
            None => {
                Poll::Ready(next)
            }
        }
    }
}
