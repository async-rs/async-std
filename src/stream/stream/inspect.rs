use std::marker::PhantomData;
use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A stream that does something with each element of another stream.
    #[derive(Debug)]
    pub struct Inspect<S, F, T> {
        #[pin]
        stream: S,
        f: F,
        __t: PhantomData<T>,
    }
}

impl<S, F, T> Inspect<S, F, T> {
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

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let next = futures_core::ready!(this.stream.as_mut().poll_next(cx));

        Poll::Ready(next.and_then(|x| {
            (this.f)(&x);
            Some(x)
        }))
    }
}
