use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct LastFuture<'a, S> 
where
    S: Stream
{
    stream: &'a mut S,
    last: Option<S::Item>,
}

impl<S: Unpin> Unpin for LastFuture<'_, S>
where
    S: Stream
{}

impl<'a, S> LastFuture<'a, S> 
where
    S: Stream
{
    //pin_utils::unsafe_pinned!(stream: S);

    pub(crate) fn new(stream: &'a mut S) -> Self
    {
        LastFuture { stream, last: None }
    }
}

impl<'a, S> Future for LastFuture<'a, S>
where
    S: Stream + Unpin + Sized,
{
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let next = futures_core::ready!(Pin::new(&mut *self.stream).poll_next(cx));

        match next {
            Some(v) => {
                self.last = Some(v);
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            None => return Poll::Ready(self.last),
        }
    }
}
