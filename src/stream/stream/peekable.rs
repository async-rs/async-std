use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Peekable<S: Stream> {
    stream: S,
    peeked: Option<PeekFuture<Option<S::Item>>>,
}

pub struct PeekFuture<'a, T: Unpin + ?Sized> {
    pub(crate) stream: &'a T,
}

impl<T: Stream + Unpin + ?Sized> Future for PeekFuture<'_, T> {
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

    pub fn peek(&mut self) -> PeekFuture<Option<&S::Item>> {
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
            Some(_) =>  {
                let v = Poll::Ready(self.as_mut().peeked().take().unwrap());
                self.peeked = None;
                v
            },
            None => {
                let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));
                match next {
                    Some(v) => Poll::Ready(Some(v)),
                    None => Poll::Ready(None),
                }
            },
        }
    }
}
