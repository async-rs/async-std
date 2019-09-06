use crate::future::Future;
use crate::task::{Context, Poll};

use std::marker::PhantomData;
use std::pin::Pin;

#[derive(Debug)]
pub struct AllFuture<'a, S, F, T>
where
    F: FnMut(T) -> bool,
{
    pub(crate) stream: &'a mut S,
    pub(crate) f: F,
    pub(crate) result: bool,
    pub(crate) __item: PhantomData<T>,
}

impl<'a, S, F, T> AllFuture<'a, S, F, T>
where
    F: FnMut(T) -> bool,
{
    pin_utils::unsafe_pinned!(stream: &'a mut S);
    pin_utils::unsafe_unpinned!(result: bool);
    pin_utils::unsafe_unpinned!(f: F);
}

impl<S, F> Future for AllFuture<'_, S, F, S::Item>
where
    S: futures_core::stream::Stream + Unpin + Sized,
    F: FnMut(S::Item) -> bool,
{
    type Output = bool;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use futures_core::stream::Stream;
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));
        match next {
            Some(v) => {
                let result = (self.as_mut().f())(v);
                *self.as_mut().result() = result;
                if result {
                    // don't forget to wake this task again to pull the next item from stream
                    cx.waker().wake_by_ref();
                    Poll::Pending
                } else {
                    Poll::Ready(false)
                }
            }
            None => Poll::Ready(self.result),
        }
    }
}
