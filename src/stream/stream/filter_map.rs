use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::stream::Stream;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FilterMap<S, F, T, B> {
    stream: S,
    f: F,
    __from: PhantomData<T>,
    __to: PhantomData<B>,
}

impl<S, F, T, B> FilterMap<S, F, T, B> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(f: F);

    pub(crate) fn new(stream: S, f: F) -> Self {
        FilterMap {
            stream,
            f,
            __from: PhantomData,
            __to: PhantomData,
        }
    }
}

impl<S, F, B> Stream for FilterMap<S, F, S::Item, B>
where
    S: Stream,
    F: FnMut(S::Item) -> Option<B>,
{
    type Item = B;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));
        match next {
            Some(v) => match (self.as_mut().f())(v) {
                Some(b) => Poll::Ready(Some(b)),
                None => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            },
            None => Poll::Ready(None),
        }
    }
}
