use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Delay<S> {
    stream: S,
    delay: futures_timer::Delay,
    delay_done: bool,
}

impl<S> Delay<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_pinned!(delay: futures_timer::Delay);
    pin_utils::unsafe_unpinned!(delay_done: bool);

    pub(super) fn new(stream: S, dur: Duration) -> Self {
        Delay {
            stream,
            delay: futures_timer::Delay::new(dur),
            delay_done: false,
        }
    }
}

impl<S> Stream for Delay<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if !self.delay_done {
            futures_core::ready!(self.as_mut().delay().poll(cx));
            *self.as_mut().delay_done() = true;
        }

        self.as_mut().stream().poll_next(cx)
    }
}
