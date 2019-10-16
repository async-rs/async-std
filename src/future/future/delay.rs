use std::pin::Pin;
use std::time::Duration;

use futures_timer::Delay;

use crate::future::Future;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[derive(Debug)]
pub struct DelayFuture<F> {
    future: F,
    delay: Delay,
}

impl<F> DelayFuture<F> {
    pin_utils::unsafe_pinned!(future: F);
    pin_utils::unsafe_pinned!(delay: Delay);

    pub fn new(future: F, dur: Duration) -> DelayFuture<F> {
        let delay = Delay::new(dur);

        DelayFuture { future, delay }
    }
}

impl<F: Future> Future for DelayFuture<F> {
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().delay().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(_) => match self.future().poll(cx) {
                Poll::Ready(v) => Poll::Ready(v),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}
