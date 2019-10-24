use crate::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Delay<F> {
    fut: F,
    delay: futures_timer::Delay,
    delay_done: bool,
}

impl<F> Delay<F> {
    pin_utils::unsafe_pinned!(fut: F);
    pin_utils::unsafe_pinned!(delay: futures_timer::Delay);
    pin_utils::unsafe_unpinned!(delay_done: bool);

    pub(super) fn new(fut: F, dur: Duration) -> Self {
        Delay {
            fut,
            delay: futures_timer::Delay::new(dur),
            delay_done: false,
        }
    }
}

impl<F> Future for Delay<F>
where
    F: Future
{
    type Output = F::Output;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.delay_done {
            futures_core::ready!(self.as_mut().delay().poll(cx));
            *self.as_mut().delay_done() = true;
        }

        self.as_mut().fut().poll(cx)
    }
}
