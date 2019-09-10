use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

#[allow(missing_debug_implementations)]
pub struct FindMapFuture<'a, S, F, T, B> {
    stream: &'a mut S,
    f: F,
    __b: PhantomData<B>,
    __t: PhantomData<T>,
}

impl<'a, S, B, F, T> FindMapFuture<'a, S, F, T, B> {
    pin_utils::unsafe_pinned!(stream: &'a mut S);
    pin_utils::unsafe_unpinned!(f: F);

    pub(super) fn new(stream: &'a mut S, f: F) -> Self {
        FindMapFuture {
            stream,
            f,
            __b: PhantomData,
            __t: PhantomData,
        }
    }
}

impl<'a, S, B, F> futures_core::future::Future for FindMapFuture<'a, S, F, S::Item, B>
where
    S: futures_core::stream::Stream + Unpin + Sized,
    F: FnMut(S::Item) -> Option<B>,
{
    type Output = Option<B>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use futures_core::stream::Stream;

        let item = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match item {
            Some(v) => match (self.as_mut().f())(v) {
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
