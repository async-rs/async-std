use std::error::Error;
use std::fmt;
use std::pin::Pin;
use std::time::Duration;

use futures_timer::Delay;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[derive(Debug)]
pub struct TimeoutStream<S> {
    stream: S,
    delay: Delay,
}

impl<S> TimeoutStream<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_pinned!(delay: Delay);

    pub fn new(stream: S, dur: Duration) -> TimeoutStream<S> {
        let delay = Delay::new(dur);

        TimeoutStream { stream, delay }
    }
}

impl<S: Stream> Stream for TimeoutStream<S> {
    type Item = Result<S::Item, TimeoutError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.as_mut().stream().poll_next(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some(Ok(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => match self.delay().poll(cx) {
                Poll::Ready(_) => Poll::Ready(Some(Err(TimeoutError { _private: () }))),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

/// An error returned when a stream times out.
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
#[cfg(any(feature = "unstable", feature = "docs"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeoutError {
    _private: (),
}

impl Error for TimeoutError {}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "stream has timed out".fmt(f)
    }
}
