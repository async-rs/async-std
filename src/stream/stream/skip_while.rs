use std::marker::PhantomData;
use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A stream to skip elements of another stream based on a predicate.
    ///
    /// This `struct` is created by the [`skip_while`] method on [`Stream`]. See its
    /// documentation for more.
    ///
    /// [`skip_while`]: trait.Stream.html#method.skip_while
    /// [`Stream`]: trait.Stream.html
    #[derive(Debug)]
    pub struct SkipWhile<S, P, T> {
        #[pin]
        stream: S,
        predicate: Option<P>,
        __t: PhantomData<T>,
    }
}

impl<S, P, T> SkipWhile<S, P, T> {
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

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            let next = futures_core::ready!(this.stream.as_mut().poll_next(cx));

            match next {
                Some(v) => match this.predicate {
                    Some(p) => {
                        if !p(&v) {
                            *this.predicate = None;
                            return Poll::Ready(Some(v));
                        }
                    }
                    None => return Poll::Ready(Some(v)),
                },
                None => return Poll::Ready(None),
            }
        }
    }
}
