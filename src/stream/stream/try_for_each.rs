use std::marker::PhantomData;
use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct TryForEachFuture<S, F, T, R> {
        #[pin]
        stream: S,
        f: F,
        __from: PhantomData<T>,
        __to: PhantomData<R>,
    }
}

impl<S, F, T, R> TryForEachFuture<S, F, T, R> {
    pub(crate) fn new(stream: S, f: F) -> Self {
        TryForEachFuture {
            stream,
            f,
            __from: PhantomData,
            __to: PhantomData,
        }
    }
}

impl<S, F, E> Future for TryForEachFuture<S, F, S::Item, E>
where
    S: Stream,
    S::Item: std::fmt::Debug,
    F: FnMut(S::Item) -> Result<(), E>,
{
    type Output = Result<(), E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            let item = futures_core::ready!(this.stream.as_mut().poll_next(cx));

            match item {
                None => return Poll::Ready(Ok(())),
                Some(v) => {
                    let res = (this.f)(v);
                    if let Err(e) = res {
                        return Poll::Ready(Err(e));
                    }
                }
            }
        }
    }
}
