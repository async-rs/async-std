use crate::task::{Context, Poll};
use std::marker::PhantomData;
use std::pin::Pin;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FindFuture<'a, S, P, T> {
    stream: &'a mut S,
    p: P,
    __t: PhantomData<T>,
}

impl<'a, S, P, T> FindFuture<'a, S, P, T> {
    pin_utils::unsafe_pinned!(stream: &'a mut S);
    pin_utils::unsafe_unpinned!(p: P);

    pub(super) fn new(stream: &'a mut S, p: P) -> Self {
        FindFuture {
            stream,
            p,
            __t: PhantomData,
        }
    }
}

impl<'a, S, P> futures_core::future::Future for FindFuture<'a, S, P, S::Item>
where
    S: futures_core::stream::Stream + Unpin + Sized,
    P: FnMut(&S::Item) -> bool,
{
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use futures_core::stream::Stream;

        let item = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match item {
            Some(v) => match (self.as_mut().p())(&v) {
                true => Poll::Ready(Some(v)),
                false => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            },
            None => Poll::Ready(None),
        }
    }
}
