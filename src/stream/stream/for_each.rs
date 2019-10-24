use std::marker::PhantomData;
use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct ForEachFuture<S, F, T> {
        #[pin]
        stream: S,
        f: F,
        __t: PhantomData<T>,
    }
}

impl<S, F, T> ForEachFuture<S, F, T> {
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            let next = futures_core::ready!(this.stream.as_mut().poll_next(cx));

            match next {
                Some(v) => (this.f)(v),
                None => return Poll::Ready(()),
            }
        }
    }
}
