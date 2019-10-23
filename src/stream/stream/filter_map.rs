use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

use crate::stream::Stream;

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct FilterMap<S, F, T, B> {
        #[pin]
        stream: S,
        f: F,
        __from: PhantomData<T>,
        __to: PhantomData<B>,
    }
}

impl<S, F, T, B> FilterMap<S, F, T, B> {
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

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let next = futures_core::ready!(this.stream.poll_next(cx));
        match next {
            Some(v) => match (this.f)(v) {
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
