use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct AnyFuture<'a, S, F, Fut, T> {
    pub(crate) stream: &'a mut S,
    pub(crate) f: F,
    pub(crate) result: bool,
    pub(crate) future: Option<Fut>,
    pub(crate) _marker: PhantomData<T>,
}

impl<'a, S, F, Fut, T> AnyFuture<'a, S, F, Fut, T> {
    pin_utils::unsafe_unpinned!(f: F);
    pin_utils::unsafe_pinned!(future: Option<Fut>);
    pin_utils::unsafe_pinned!(stream: &'a mut S);
}

impl<S, F, Fut> Future for AnyFuture<'_, S, F, Fut, S::Item>
where
    S: Stream + Unpin + Sized,
    F: FnMut(S::Item) -> Fut,
    Fut: Future<Output = bool>,
{
    type Output = bool;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.future.is_some() {
                false => {
                    let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));
                    match next {
                        Some(v) => {
                            let fut = (self.as_mut().f())(v);
                            self.as_mut().future().set(Some(fut));
                        }
                        None => return Poll::Ready(self.result),
                    }
                }
                true => {
                    let res =
                        futures_core::ready!(self.as_mut().future().as_pin_mut().unwrap().poll(cx));

                    self.as_mut().future().set(None);
                    if res {
                        return Poll::Ready(true);
                    }
                }
            }
        }
    }
}
