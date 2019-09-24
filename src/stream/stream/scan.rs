use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream to maintain state while polling another stream.
#[derive(Debug)]
pub struct Scan<S, St, F> {
    stream: S,
    state_f: (St, F),
}

impl<S, St, F> Scan<S, St, F> {
    pub(crate) fn new(stream: S, initial_state: St, f: F) -> Self {
        Self {
            stream,
            state_f: (initial_state, f),
        }
    }

    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(state_f: (St, F));
}

impl<S: Unpin, St, F> Unpin for Scan<S, St, F> {}

impl<S, St, F, B> Stream for Scan<S, St, F>
where
    S: Stream,
    F: FnMut(&mut St, S::Item) -> Option<B>,
{
    type Item = B;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<B>> {
        let poll_result = self.as_mut().stream().poll_next(cx);
        poll_result.map(|item| {
            item.and_then(|item| {
                let (state, f) = self.as_mut().state_f();
                f(state, item)
            })
        })
    }
}
