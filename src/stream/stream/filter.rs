use std::marker::PhantomData;
use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream to filter elements of another stream with a predicate.
#[derive(Debug)]
pub struct Filter<S, P, T> {
    stream: S,
    predicate: P,
    __t: PhantomData<T>,
}

impl<S, P, T> Filter<S, P, T> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(predicate: P);

    pub(super) fn new(stream: S, predicate: P) -> Self {
        Filter {
            stream,
            predicate,
            __t: PhantomData,
        }
    }
}

impl<S, P> Stream for Filter<S, P, S::Item>
where
    S: Stream,
    P: FnMut(&S::Item) -> bool,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match next {
            Some(v) => match (self.as_mut().predicate())(&v) {
                true => Poll::Ready(Some(v)),
                false => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            },
            None => Poll::Ready(None),
        }
    }
}
