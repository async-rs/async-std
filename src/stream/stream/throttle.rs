use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use futures_timer::Delay;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream that only yields one element once every `duration`, and drops all others.
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
        match self.as_mut().stream().poll_next(cx) {
            Poll::Ready(v) => match self.as_mut().delay().as_pin_mut() {
                None => {
                    *self.as_mut().delay() = Some(Delay::new(self.duration));
                    Poll::Ready(v)
                }
                Some(d) => match d.poll(cx) {
                    Poll::Ready(_) => {
                        *self.as_mut().delay() = Some(Delay::new(self.duration));
                        Poll::Ready(v)
                    }
                    Poll::Pending => Poll::Pending,
                },
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
