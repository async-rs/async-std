use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct LastFuture<S, T> {
    stream: S,
    last: Option<T>,
}

impl<S, T> LastFuture<S, T> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(last: Option<T>);

    pub(crate) fn new(stream: S) -> Self {
        LastFuture { stream, last: None }
    }
}

impl<S> Future for LastFuture<S, S::Item>
where
    S: Stream + Unpin + Sized,
    S::Item: Copy,
{
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match next {
            Some(new) => {
                cx.waker().wake_by_ref();
                *self.as_mut().last() = Some(new);
                Poll::Pending
            }
            None => Poll::Ready(self.last),
        }
    }
}
