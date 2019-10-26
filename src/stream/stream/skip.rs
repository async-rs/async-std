use std::pin::Pin;
use std::task::{Context, Poll};

use pin_project_lite::pin_project;

use crate::stream::Stream;

pin_project! {
    /// A stream to skip first n elements of another stream.
    #[derive(Debug)]
    pub struct Skip<S> {
        #[pin]
        stream: S,
        n: usize,
    }
}

impl<S> Skip<S> {
    pub(crate) fn new(stream: S, n: usize) -> Self {
        Skip { stream, n }
    }
}

impl<S> Stream for Skip<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            let next = futures_core::ready!(this.stream.as_mut().poll_next(cx));

            match next {
                Some(v) => match *this.n {
                    0 => return Poll::Ready(Some(v)),
                    _ => *this.n -= 1,
                },
                None => return Poll::Ready(None),
            }
        }
    }
}
