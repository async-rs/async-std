use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ForEachFuture<S, F, Fut, T> {
    stream: S,
    f: F,
    future: Option<Fut>,
    __t: PhantomData<T>,
}

impl<S, F, Fut, T> ForEachFuture<S, F, Fut, T> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_pinned!(future: Option<Fut>);
    pin_utils::unsafe_unpinned!(f: F);

    pub(super) fn new(stream: S, f: F) -> Self {
        ForEachFuture {
            stream,
            f,
            future: None,
            __t: PhantomData,
        }
    }
}

impl<S, F, Fut> Future for ForEachFuture<S, F, Fut, S::Item>
where
    S: Stream + Sized,
    F: FnMut(S::Item) -> Fut,
    Fut: Future<Output = ()>,
{
    type Output = ();

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
                        None => return Poll::Ready(()),
                    }
                }
                true => {
                    let _ =
                        futures_core::ready!(self.as_mut().future().as_pin_mut().unwrap().poll(cx));

                    self.as_mut().future().set(None);
                }
            }
        }
    }
}
