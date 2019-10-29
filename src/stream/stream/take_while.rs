use std::marker::PhantomData;
use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A stream that yields elements based on a predicate.
    ///
    /// This `struct` is created by the [`take_while`] method on [`Stream`]. See its
    /// documentation for more.
    ///
    /// [`take_while`]: trait.Stream.html#method.take_while
    /// [`Stream`]: trait.Stream.html
    #[derive(Debug)]
    pub struct TakeWhile<S, P, T> {
        #[pin]
        stream: S,
        predicate: P,
        __t: PhantomData<T>,
    }
}

impl<S, P, T> TakeWhile<S, P, T> {
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

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let next = futures_core::ready!(this.stream.poll_next(cx));

        match next {
            Some(v) if (this.predicate)(&v) => Poll::Ready(Some(v)),
            Some(_) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            None => Poll::Ready(None),
        }
    }
}
