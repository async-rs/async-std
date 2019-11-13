use std::pin::Pin;
use std::future::Future;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct CountFuture<S> {
        #[pin]
        stream: S,
        count: usize,
    }
}

impl<S> CountFuture<S> {
    pub(super) fn new(stream: S) -> Self {
        Self {
            stream,
            count: 0,
        }
    }
}

impl<S> Future for CountFuture<S>
where
    S: Stream,
{
    type Output = usize;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let next = futures_core::ready!(this.stream.poll_next(cx));

        match next {
            Some(_) => {
                cx.waker().wake_by_ref();
                *this.count += 1;
                Poll::Pending
            }
            None => Poll::Ready(*this.count),
        }
    }
}
