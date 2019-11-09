use std::marker::PhantomData;
use std::pin::Pin;
use std::future::Future;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct FoldFuture<S, F, T, B> {
        #[pin]
        stream: S,
        f: F,
        acc: Option<B>,
        __t: PhantomData<T>,
    }
}

impl<S, F, T, B> FoldFuture<S, F, T, B> {
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
    S: Stream + Sized,
    F: FnMut(B, S::Item) -> B,
{
    type Output = B;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            let next = futures_core::ready!(this.stream.as_mut().poll_next(cx));

            match next {
                Some(v) => {
                    let old = this.acc.take().unwrap();
                    let new = (this.f)(old, v);
                    *this.acc = Some(new);
                }
                None => return Poll::Ready(this.acc.take().unwrap()),
            }
        }
    }
}
