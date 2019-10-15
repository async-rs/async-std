use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct TryFoldFuture<S, F, T> {
    stream: S,
    f: F,
    acc: Option<T>,
    __t: PhantomData<T>,
}

impl<S, F, T> TryFoldFuture<S, F, T> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(f: F);
    pin_utils::unsafe_unpinned!(acc: Option<T>);

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

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

            match next {
                Some(v) => {
                    let old = self.as_mut().acc().take().unwrap();
                    let new = (self.as_mut().f())(old, v);

                    match new {
                        Ok(o) => {
                            *self.as_mut().acc() = Some(o);
                        }
                        Err(e) => return Poll::Ready(Err(e)),
                    }
                }
                None => return Poll::Ready(Ok(self.as_mut().acc().take().unwrap())),
            }
        }
    }
}
