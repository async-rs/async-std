use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FoldFuture<S, F, Fut, T, B> {
    stream: S,
    f: F,
    future: Option<Fut>,
    acc: Option<B>,
    __t: PhantomData<T>,
}

impl<S, F, Fut, T, B> FoldFuture<S, F, Fut, T, B> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(f: F);
    pin_utils::unsafe_unpinned!(acc: Option<B>);
    pin_utils::unsafe_pinned!(future: Option<Fut>);

    pub(super) fn new(stream: S, init: B, f: F) -> Self {
        FoldFuture {
            stream,
            f,
            future: None,
            acc: Some(init),
            __t: PhantomData,
        }
    }
}

impl<S, F, Fut, B> Future for FoldFuture<S, F, Fut, S::Item, B>
where
    S: Stream + Sized,
    F: FnMut(B, S::Item) -> Fut,
    Fut: Future<Output = B>,
{
    type Output = B;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.future.is_some() {
                false => {
                    let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

                    match next {
                        Some(v) => {
                            let old = self.as_mut().acc().take().unwrap();
                            let fut = (self.as_mut().f())(old, v);
                            self.as_mut().future().set(Some(fut));
                        }
                        None => return Poll::Ready(self.as_mut().acc().take().unwrap()),
                    }
                }
                true => {
                    let res =
                        futures_core::ready!(self.as_mut().future().as_pin_mut().unwrap().poll(cx));
                    self.as_mut().future().set(None);
                    *self.as_mut().acc() = Some(res);
                }
            }
        }
    }
}
