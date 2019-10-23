use std::time::Duration;
use std::pin::Pin;
use std::future::Future;

use futures_timer::Delay;

use crate::stream::Stream;
use crate::task::{Context, Poll};

// TODO: derives
pub struct Debounce<S: Stream> {
    stream: S,
    duration: Duration,
    state: DebounceState<S::Item>,
}

enum DebounceState<T> {
    /// Waiting for a value from the inner stream
    Start,
    /// Waiting for the delay timer to expire
    Waiting {
        delayed: Delayed<T>,
        last: bool,
    },
    /// Inner stream has finished
    Done,
}

/// A value to be delayed by an arbitrary duration. Implementation detail.
struct Delayed<T> {
    delay: Delay,
    value: Option<T>,
}

impl<T> Delayed<T> {
    fn new(duration: Duration, value: T) -> Self {
        Self {
            delay: Delay::new(duration),
            value: Some(value),
        }
    }

    pin_utils::unsafe_pinned!(delay: Delay);
    pin_utils::unsafe_unpinned!(value: Option<T>);
}

impl<T> Future for Delayed<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().delay().poll(cx) {
            // Still waiting for delay timer...
            Poll::Pending => Poll::Pending,
            // Ding ding!
            Poll::Ready(_) => {
                // Only return the value once
                let extracted = self.as_mut().value().take()
                    .expect("future poll()'ed too many times");
                Poll::Ready(extracted)
            },
        }
    }
}

impl<T> DebounceState<T> {
    fn delayed(self: Pin<&mut Self>) -> Option<Pin<&mut Delayed<T>>> {
        let inner = unsafe { self.get_unchecked_mut() };
        match *inner {
            DebounceState::Waiting { ref mut delayed, .. } => {
                Some(unsafe { Pin::new_unchecked(delayed) })
            },
            _ => None,
        }
    }

    fn mark_last(&mut self) {
        if let DebounceState::Waiting { ref mut last, .. } = *self {
            *last = true;
        } else {
            panic!("DebounceState::mark_last() called in non-waiting state");
        }
    }

    fn is_last(&self) -> bool {
        if let DebounceState::Waiting { ref last, .. } = *self {
            *last
        } else {
            panic!("DebounceState::is_last() called in non-waiting state");
        }
    }
}

impl<S: Stream> Debounce<S> {
    pub fn new(stream: S, duration: Duration) -> Self {
        Self {
            stream,
            duration,
            state: DebounceState::Start,
        }
    }

    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_pinned!(state: DebounceState<S::Item>);
}

impl<S: Stream> Stream for Debounce<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {

        let duration = self.as_ref().get_ref().duration;

        let inner = self.as_mut().stream().poll_next(cx);
        let mut state = self.as_mut().state();

        match *state.as_ref().get_ref() {
            DebounceState::Start => {
                if let Poll::Ready(val) = inner {
                    let state = unsafe { state.get_unchecked_mut() };
                    if let Some(val) = val {
                        // We have our first value, so let's begin waiting
                        *state = DebounceState::Waiting{
                            delayed: Delayed::new(duration, val),
                            last: false,
                        };
                    } else {
                        // The inner stream finished, so we should finish too
                        *state = DebounceState::Done;
                        return Poll::Ready(None);
                    }
                }

                // No value yet...
                Poll::Pending
            },
            DebounceState::Waiting { .. } => {
                if let Poll::Ready(val) = inner {
                    let state = unsafe { state.as_mut().get_unchecked_mut() };
                    if let Some(val) = val {
                        // Reset the wait period with the new value
                        *state = DebounceState::Waiting{
                            delayed: Delayed::new(duration, val),
                            last: false,
                        };
                    } else {
                        // Mark the current value as the last
                        state.mark_last();
                    }
                }

                let delayed = state.as_mut().delayed()
                    .expect("assuming DebounceState::Waiting");

                match delayed.poll(cx) {
                    Poll::Ready(val) => {
                        let state = unsafe { state.as_mut().get_unchecked_mut() };
                        *state = if state.is_last() {
                            // We are immediately ready after this with None
                            cx.waker().wake_by_ref();
                            DebounceState::Done
                        } else {
                            DebounceState::Start
                        };

                        Poll::Ready(Some(val))
                    },
                    Poll::Pending => Poll::Pending,
                }
            },
            DebounceState::Done => {
                Poll::Ready(None)
            },
        }
    }
}
