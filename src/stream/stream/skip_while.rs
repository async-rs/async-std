use std::marker::PhantomData;
use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream to skip elements of another stream based on a predicate.
#[derive(Debug)]
pub struct SkipWhile<S, P, T> {
    stream: S,
    predicate: Option<P>,
    __t: PhantomData<T>,
}

impl<S, P, T> SkipWhile<S, P, T> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(predicate: Option<P>);

    pub(crate) fn new(stream: S, predicate: P) -> Self {
        SkipWhile {
            stream,
            predicate: Some(predicate),
            __t: PhantomData,
        }
    }
}

impl<S, P> Stream for SkipWhile<S, P, S::Item>
where
    S: Stream,
    P: FnMut(&S::Item) -> bool,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

            match next {
                Some(v) => match self.as_mut().predicate() {
                    Some(p) => match p(&v) {
                        true => (),
                        false => {
                            *self.as_mut().predicate() = None;
                            return Poll::Ready(Some(v));
                        }
                    },
                    None => return Poll::Ready(Some(v)),
                },
                None => return Poll::Ready(None),
            }
        }
    }
}
