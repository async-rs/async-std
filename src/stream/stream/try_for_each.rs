use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct TryForEeachFuture<S, F, T, R> {
    stream: S,
    f: F,
    __from: PhantomData<T>,
    __to: PhantomData<R>,
}

impl<S, F, T, R> TryForEeachFuture<S, F, T, R> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(f: F);

    pub(crate) fn new(stream: S, f: F) -> Self {
        TryForEeachFuture {
            stream,
            f,
            __from: PhantomData,
            __to: PhantomData,
        }
    }
}

impl<S, F, E> Future for TryForEeachFuture<S, F, S::Item, E>
where
    S: Stream,
    S::Item: std::fmt::Debug,
    F: FnMut(S::Item) -> Result<(), E>,
{
    type Output = Result<(), E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let item = futures_core::ready!(self.as_mut().stream().poll_next(cx));

            match item {
                None => return Poll::Ready(Ok(())),
                Some(v) => {
                    let res = (self.as_mut().f())(v);
                    if let Err(e) = res {
                        return Poll::Ready(Err(e));
                    }
                }
            }
        }
    }
}
