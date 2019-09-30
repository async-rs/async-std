use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ForEachFuture<S, F, T> {
    stream: S,
    f: F,
    __t: PhantomData<T>,
}

impl<S, F, T> ForEachFuture<S, F, T> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(f: F);

    pub(super) fn new(stream: S, f: F) -> Self {
        ForEachFuture {
            stream,
            f,
            __t: PhantomData,
        }
    }
}

impl<S, F> Future for ForEachFuture<S, F, S::Item>
where
    S: Stream + Sized,
    F: FnMut(S::Item),
{
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

            match next {
                Some(v) => (self.as_mut().f())(v),
                None => return Poll::Ready(()),
            }
        }
    }
}
