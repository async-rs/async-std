use std::marker::PhantomData;
use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

#[derive(Debug)]
pub struct TakeWhile<S, P, T> {
    stream: S,
    predicate: P,
    __t: PhantomData<T>,
}

impl<S, P, T> TakeWhile<S, P, T> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(predicate: P);

    pub(super) fn new(stream: S, predicate: P) -> Self {
        TakeWhile {
            stream,
            predicate,
            __t: PhantomData,
        }
    }
}

impl<S, P> Stream for TakeWhile<S, P, S::Item>
where
    S: Stream,
    P: FnMut(&S::Item) -> bool,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match next {
            Some(v) if (self.as_mut().predicate())(&v) => Poll::Ready(Some(v)),
            Some(_) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            None => Poll::Ready(None),
        }
    }
}
