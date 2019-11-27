use std::future::Future;
use std::pin::Pin;

use crate::future::IntoFuture;
use crate::task::{ready, Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FlattenFuture<Fut1, Fut2> {
    state: State<Fut1, Fut2>,
}

#[derive(Debug)]
enum State<Fut1, Fut2> {
    First(Fut1),
    Second(Fut2),
    Empty,
}

impl<Fut1, Fut2> FlattenFuture<Fut1, Fut2> {
    pub(crate) fn new(future: Fut1) -> FlattenFuture<Fut1, Fut2> {
        FlattenFuture {
            state: State::First(future),
        }
    }
}

impl<Fut1> Future for FlattenFuture<Fut1, <Fut1::Output as IntoFuture>::Future>
where
    Fut1: Future,
    Fut1::Output: IntoFuture,
{
    type Output = <Fut1::Output as IntoFuture>::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self { state } = unsafe { self.get_unchecked_mut() };
        loop {
            match state {
                State::First(fut1) => {
                    let fut2 = ready!(unsafe { Pin::new_unchecked(fut1) }.poll(cx)).into_future();
                    *state = State::Second(fut2);
                }
                State::Second(fut2) => {
                    let v = ready!(unsafe { Pin::new_unchecked(fut2) }.poll(cx));
                    *state = State::Empty;
                    return Poll::Ready(v);
                }
                State::Empty => panic!("polled a completed future"),
            }
        }
    }
}
