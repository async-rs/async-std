use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Map<S, F, T, Fut, B> {
    stream: S,
    f: F,
    future: Option<Fut>,
    __from: PhantomData<T>,
    __to: PhantomData<B>,
}

impl<S, F, T, Fut, B> Map<S, F, T, Fut, B> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(f: F);
    pin_utils::unsafe_pinned!(future: Option<Fut>);

    pub(crate) fn new(stream: S, f: F) -> Self {
        Map {
            stream,
            f,
            future: None,
            __from: PhantomData,
            __to: PhantomData,
        }
    }
}

impl<S, F, Fut, B> Stream for Map<S, F, S::Item, Fut, B>
where
    S: Stream,
    F: FnMut(S::Item) -> Fut,
    Fut: Future<Output = B>,
{
    type Item = B;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.future.is_some() {
                false => {
                    let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));
                    match next {
                        Some(v) => {
                            let fut = (self.as_mut().f())(v);
                            self.as_mut().future().set(Some(fut));
                        }
                        None => {
                            return Poll::Ready(None);
                        }
                    }
                }
                true => {
                    let res =
                        futures_core::ready!(self.as_mut().future().as_pin_mut().unwrap().poll(cx));
                    self.as_mut().future().set(None);
                    return Poll::Ready(Some(res));
                }
            }
        }
    }
}
