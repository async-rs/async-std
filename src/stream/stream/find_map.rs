use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::future::Future;

use crate::stream::Stream;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FindMapFuture<'a, S, F, T, B> {
    stream: &'a mut S,
    f: F,
    __b: PhantomData<B>,
    __t: PhantomData<T>,
}

impl<'a, S, B, F, T> FindMapFuture<'a, S, F, T, B> {
    pub(super) fn new(stream: &'a mut S, f: F) -> Self {
        FindMapFuture {
            stream,
            f,
            __b: PhantomData,
            __t: PhantomData,
        }
    }
}

impl<S: Unpin, F, T, B> Unpin for FindMapFuture<'_, S, F, T, B> {}

impl<'a, S, B, F> Future for FindMapFuture<'a, S, F, S::Item, B>
where
    S: Stream + Unpin + Sized,
    F: FnMut(S::Item) -> Option<B>,
{
    type Output = Option<B>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let item = futures_core::ready!(Pin::new(&mut *self.stream).poll_next(cx));

        match item {
            Some(v) => match (&mut self.f)(v) {
                Some(v) => Poll::Ready(Some(v)),
                None => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            },
            None => Poll::Ready(None),
        }
    }
}
