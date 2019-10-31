use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct PositionFuture<S, P> {
        #[pin]
        stream: S,
        predicate: P,
        index:usize,
    }
}

impl<S, P> PositionFuture<S, P> {
    pub(super) fn new(stream: S, predicate: P) -> Self {
        PositionFuture {
            stream,
            predicate,
            index: 0,
        }
    }
}

impl<S, P> Future for PositionFuture<S, P>
where
    S: Stream,
    P: FnMut(&S::Item) -> bool,
{
    type Output = Option<usize>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let next = futures_core::ready!(this.stream.poll_next(cx));

        match next {
            Some(v) if (this.predicate)(&v) => Poll::Ready(Some(*this.index)),
            Some(_) => {
                cx.waker().wake_by_ref();
                *this.index += 1;
                Poll::Pending
            }
            None => Poll::Ready(None),
        }
    }
}
