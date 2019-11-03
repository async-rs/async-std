use futures_core::ready;
use std::pin::Pin;

use crate::future::Future;
use crate::future::IntoFuture;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[derive(Debug)]
pub enum FlattenFuture<Fut1, Fut2> {
    First(Fut1),
    Second(Fut2),
    Empty,
}

impl<Fut1, Fut2> FlattenFuture<Fut1, Fut2> {
    pub fn new(future: Fut1) -> FlattenFuture<Fut1, Fut2> {
        FlattenFuture::First(future)
    }
}

impl<Fut1> Future for FlattenFuture<Fut1, <Fut1::Output as IntoFuture>::Future>
where
    Fut1: Future,
    Fut1::Output: IntoFuture,
{
    type Output = <Fut1::Output as IntoFuture>::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        loop {
            match this {
                FlattenFuture::First(fut1) => {
                    let fut2 = ready!(unsafe { Pin::new_unchecked(fut1) }.poll(cx)).into_future();
                    *this = FlattenFuture::Second(fut2);
                }
                FlattenFuture::Second(fut2) => {
                    let v = ready!(unsafe { Pin::new_unchecked(fut2) }.poll(cx));
                    *this = FlattenFuture::Empty;
                    return Poll::Ready(v);
                }
                FlattenFuture::Empty => unreachable!(),
            }
        }
    }
}
