use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FindFuture<'a, S, P, T> {
    stream: &'a mut S,
    p: P,
    __t: PhantomData<T>,
}

impl<'a, S, P, T> FindFuture<'a, S, P, T> {
    pub(super) fn new(stream: &'a mut S, p: P) -> Self {
        FindFuture {
            stream,
            p,
            __t: PhantomData,
        }
    }
}

impl<S: Unpin, P, T> Unpin for FindFuture<'_, S, P, T> {}

impl<'a, S, P> Future for FindFuture<'a, S, P, S::Item>
where
    S: Stream + Unpin + Sized,
    P: FnMut(&S::Item) -> bool,
{
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let item = futures_core::ready!(Pin::new(&mut *self.stream).poll_next(cx));

        match item {
            Some(v) if (&mut self.p)(&v) => Poll::Ready(Some(v)),
            Some(_) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            None => Poll::Ready(None),
        }
    }
}
