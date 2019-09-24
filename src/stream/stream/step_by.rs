use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream that steps a given amount of elements of another stream.
#[derive(Debug)]
pub struct StepBy<S> {
    stream: S,
    step: usize,
    i: usize,
}

impl<S> StepBy<S> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(step: usize);
    pin_utils::unsafe_unpinned!(i: usize);

    pub(crate) fn new(stream: S, step: usize) -> Self {
        StepBy {
            stream,
            step: step.checked_sub(1).unwrap(),
            i: 0,
        }
    }
}

impl<S> Stream for StepBy<S>
where
    S: Stream,
{
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

            match next {
                Some(v) => match self.i {
                    0 => {
                        *self.as_mut().i() = self.step;
                        return Poll::Ready(Some(v));
                    }
                    _ => *self.as_mut().i() -= 1,
                },
                None => return Poll::Ready(None),
            }
        }
    }
}
