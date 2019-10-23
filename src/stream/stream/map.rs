use std::marker::PhantomData;
use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct Map<S, F, T, B> {
        #[pin]
        stream: S,
        f: F,
        __from: PhantomData<T>,
        __to: PhantomData<B>,
    }
}

impl<S, F, T, B> Map<S, F, T, B> {
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

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let next = futures_core::ready!(this.stream.poll_next(cx));
        Poll::Ready(next.map(this.f))
    }
}
