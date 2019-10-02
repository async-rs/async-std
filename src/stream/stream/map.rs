use std::marker::PhantomData;
use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Map<S, F, T, B> {
    stream: S,
    f: F,
    __from: PhantomData<T>,
    __to: PhantomData<B>,
}

impl<S, F, T, B> Map<S, F, T, B> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(f: F);

    pub(crate) fn new(stream: S, f: F) -> Self {
        Map {
            stream,
            f,
            __from: PhantomData,
            __to: PhantomData,
        }
    }
}

impl<S, F, B> Stream for Map<S, F, S::Item, B>
where
    S: Stream,
    F: FnMut(S::Item) -> B,
{
    type Item = B;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));
        Poll::Ready(next.map(self.as_mut().f()))
    }
}
