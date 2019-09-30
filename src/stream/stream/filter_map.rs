use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::future::Future;
use crate::stream::Stream;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FilterMap<S, F, Fut, T, B> {
    stream: S,
    f: F,
    future: Option<Fut>,
    __from: PhantomData<T>,
    __to: PhantomData<B>,
}

impl<S, F, Fut, T, B> Unpin for FilterMap<S, F, Fut, T, B>
where
    S: Unpin,
    Fut: Unpin,
{
}

impl<S, F, Fut, T, B> FilterMap<S, F, Fut, T, B> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_pinned!(future: Option<Fut>);
    pin_utils::unsafe_unpinned!(f: F);

    pub(crate) fn new(stream: S, f: F) -> Self {
        FilterMap {
            stream,
            f,
            future: None,
            __from: PhantomData,
            __to: PhantomData,
        }
    }
}

impl<S, F, Fut, B> Stream for FilterMap<S, F, Fut, S::Item, B>
where
    S: Stream,
    F: FnMut(S::Item) -> Fut,
    Fut: Future<Output = Option<B>>,
{
    type Item = B;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.future.is_some() {
                false => {
                    let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));
                    match next {
                        Some(v) => {
                            let fut = self.as_mut().f()(v);
                            self.as_mut().future().set(Some(fut));
                        }
                        None => return Poll::Ready(None),
                    }
                }
                true => {
                    let res =
                        futures_core::ready!(self.as_mut().future().as_pin_mut().unwrap().poll(cx));

                    self.as_mut().future().set(None);

                    if let Some(b) = res {
                        return Poll::Ready(Some(b));
                    }
                }
            }
        }
    }
}
