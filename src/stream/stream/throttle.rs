use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use futures_timer::Delay;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream that only yields one element once every `duration`, and drops all others.
/// #[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Throttle<S, T> {
    stream: S,
    duration: Duration,
    delay: Option<Delay>,
    last: Option<T>,
}

impl<S: Unpin, T> Unpin for Throttle<S, T> {}

impl<S: Stream> Throttle<S, S::Item> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(duration: Duration);
    pin_utils::unsafe_pinned!(delay: Option<Delay>);
    pin_utils::unsafe_unpinned!(last: Option<S::Item>);

    pub(super) fn new(stream: S, duration: Duration) -> Self {
        Throttle {
            stream,
            duration,
            delay: None,
            last: None,
        }
    }
}

impl<S: Stream> Stream for Throttle<S, S::Item> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        if let Some(d) = self.as_mut().delay().as_pin_mut() {
            if d.poll(cx).is_ready() {
                if let Some(v) = self.as_mut().last().take() {
                    // Sets last to None.
                    *self.as_mut().delay() = Some(Delay::new(self.duration));
                    return Poll::Ready(Some(v));
                } else {
                    *self.as_mut().delay() = None;
                }
            }
        }

        match self.as_mut().stream().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Ready(Some(v)) => {
                if self.as_mut().delay().is_some() {
                    *self.as_mut().last() = Some(v);
                    cx.waker().wake_by_ref(); // Continue driving even though emitting Pending
                    return Poll::Pending;
                }

                *self.as_mut().delay() = Some(Delay::new(self.duration));
                Poll::Ready(Some(v))
            }
        }
    }
}
