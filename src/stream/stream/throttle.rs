use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use futures_timer::Delay;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream that only yields one element once every `duration`, and applies backpressure. Does not drop any elements.
/// #[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct Throttle<S> {
    stream: S,
    duration: Duration,
    delay: Option<Delay>,
}

impl<S: Unpin> Unpin for Throttle<S> {}

impl<S: Stream> Throttle<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(duration: Duration);
    pin_utils::unsafe_pinned!(delay: Option<Delay>);

    pub(super) fn new(stream: S, duration: Duration) -> Self {
        Throttle {
            stream,
            duration,
            delay: None,
        }
    }
}

impl<S: Stream> Stream for Throttle<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        if let Some(d) = self.as_mut().delay().as_pin_mut() {
            if d.poll(cx).is_ready() {
                *self.as_mut().delay() = None;
            } else {
                return Poll::Pending;
            }
        }

        match self.as_mut().stream().poll_next(cx) {
            Poll::Pending => {
                cx.waker().wake_by_ref(); // Continue driving even though emitting Pending
                Poll::Pending
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(v)) => {
                *self.as_mut().delay() = Some(Delay::new(self.duration));
                Poll::Ready(Some(v))
            }
        }
    }
}
