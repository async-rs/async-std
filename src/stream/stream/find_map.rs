use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::future::Future;
use crate::stream::Stream;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FindMapFuture<'a, S, F, Fut, T, B> {
    stream: &'a mut S,
    f: F,
    future: Option<Fut>,
    __b: PhantomData<B>,
    __t: PhantomData<T>,
}

impl<'a, S, F, Fut, T, B> FindMapFuture<'a, S, F, Fut, T, B> {
    pin_utils::unsafe_pinned!(stream: &'a mut S);
    pin_utils::unsafe_unpinned!(f: F);
    pin_utils::unsafe_pinned!(future: Option<Fut>);

    pub(super) fn new(stream: &'a mut S, f: F) -> Self {
        FindMapFuture {
            stream,
            f,
            future: None,
            __b: PhantomData,
            __t: PhantomData,
        }
    }
}

impl<S: Unpin, F, Fut: Unpin, T, B> Unpin for FindMapFuture<'_, S, F, Fut, T, B> {}

impl<'a, S, B, F, Fut> Future for FindMapFuture<'a, S, F, Fut, S::Item, B>
where
    S: Stream + Unpin + Sized,
    F: FnMut(S::Item) -> Fut,
    Fut: Future<Output = Option<B>>,
{
    type Output = Option<B>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.future.is_some() {
                false => {
                    let item = futures_core::ready!(self.as_mut().stream().poll_next(cx));

                    match item {
                        Some(v) => {
                            let fut = (self.as_mut().f())(v);
                            self.as_mut().future().set(Some(fut));
                        }
                        None => return Poll::Ready(None),
                    }
                }
                true => {
                    let res =
                        futures_core::ready!(self.as_mut().future().as_pin_mut().unwrap().poll(cx));
                    self.as_mut().future().set(None);

                    match res {
                        Some(v) => return Poll::Ready(Some(v)),
                        None => (),
                    }
                }
            }
        }
    }
}
