use std::marker::PhantomData;
use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream that does something with each element of another stream.
#[derive(Debug)]
pub struct Inspect<S, F, T> {
    stream: S,
    f: F,
    __t: PhantomData<T>,
}

impl<S, F, T> Inspect<S, F, T> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(f: F);

    pub(super) fn new(stream: S, f: F) -> Self {
        Inspect {
            stream,
            f,
            __t: PhantomData,
        }
    }
}

impl<S, F> Stream for Inspect<S, F, S::Item>
where
    S: Stream,
    F: FnMut(&S::Item),
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        Poll::Ready(next.and_then(|x| {
            (self.as_mut().f())(&x);
            Some(x)
        }))
    }
}
