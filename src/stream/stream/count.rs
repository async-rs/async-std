use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct CountFuture<S> {
    stream: S,
    count: usize,
}

impl<S> CountFuture<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(count: usize);

    pub(crate) fn new(stream: S) -> Self {
        CountFuture { stream, count: 0 }
    }
}

impl<S> Future for CountFuture<S>
where
    S: Stream,
{
    type Output = usize;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match next {
            Some(_) => {
                cx.waker().wake_by_ref();
                *self.as_mut().count() += 1;
                Poll::Pending
            }
            None => Poll::Ready(self.count),
        }
    }
}
