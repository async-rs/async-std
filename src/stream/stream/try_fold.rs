use std::marker::PhantomData;
use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct TryFoldFuture<S, F, T> {
        #[pin]
        stream: S,
        f: F,
        acc: Option<T>,
        __t: PhantomData<T>,
    }
}

impl<S, F, T> TryFoldFuture<S, F, T> {
    pub(super) fn new(stream: S, init: T, f: F) -> Self {
        TryFoldFuture {
            stream,
            f,
            acc: Some(init),
            __t: PhantomData,
        }
    }
}

impl<S, F, T, E> Future for TryFoldFuture<S, F, T>
where
    S: Stream + Sized,
    F: FnMut(T, S::Item) -> Result<T, E>,
{
    type Output = Result<T, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            let next = futures_core::ready!(this.stream.as_mut().poll_next(cx));

            match next {
                Some(v) => {
                    let old = this.acc.take().unwrap();
                    let new = (this.f)(old, v);

                    match new {
                        Ok(o) => *this.acc = Some(o),
                        Err(e) => return Poll::Ready(Err(e)),
                    }
                }
                None => return Poll::Ready(Ok(this.acc.take().unwrap())),
            }
        }
    }
}
