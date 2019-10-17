use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Peekable<S: Stream> {
    stream: S,
    peeked: Option<Option<S::Item>>,
}


impl<S> Peekable<S>
where
    S: Stream,
{
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(peeked: Option<Option<S::Item>>);

    pub(crate) fn new(stream: S) -> Self {
        Peekable{
            stream: stream,
            peeked: None,
        }
    }

    pub fn peek(&mut self) -> Poll<Option<&S::Item>> {
        Poll::Ready(None)
    }

}

impl<S> Stream for Peekable<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &self.peeked {
            Some(_) =>  Poll::Ready(self.as_mut().peeked().take().unwrap()) ,
            None => {
                let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));
                Poll::Ready(next)
            }
        }
    }
}
