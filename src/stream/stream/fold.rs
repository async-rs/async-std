use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FoldFuture<S, F, T, B> {
    stream: S,
    f: F,
    acc: Option<B>,
    __t: PhantomData<T>,
}

impl<S, F, T, B> FoldFuture<S, F, T, B> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(f: F);
    pin_utils::unsafe_unpinned!(acc: Option<B>);

    pub(super) fn new(stream: S, init: B, f: F) -> Self {
        FoldFuture {
            stream,
            f,
            acc: Some(init),
            __t: PhantomData,
        }
    }
}

impl<S, F, B> Future for FoldFuture<S, F, S::Item, B>
where
    S: Stream + Unpin + Sized,
    F: FnMut(B, S::Item) -> B,
{
    type Output = B;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match next {
            Some(v) => {
                cx.waker().wake_by_ref();
                let old = self.as_mut().acc().take().unwrap();
                let new = (self.as_mut().f())(old, v);
                *self.as_mut().acc() = Some(new);
                Poll::Pending
            }
            None => Poll::Ready(self.as_mut().acc().take().unwrap()),
        }
    }
}
