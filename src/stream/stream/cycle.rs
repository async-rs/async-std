use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A stream that will repeatedly yield the same list of elements
    pub struct Cycle<S, T> {
        #[pin]
        source: S,
        index: usize,
        buffer: Vec<T>,
        state: CycleState,
    }
}

#[derive(Eq, PartialEq)]
enum CycleState {
    FromStream,
    FromBuffer,
}

impl<T: Clone, S: Stream<Item = T>> Cycle<S, T> {
    pub fn new(source: S) -> Cycle<S, T> {
        Cycle {
            source,
            index: 0,
            buffer: Vec::new(),
            state: CycleState::FromStream,
        }
    }
}

impl<S, T> Stream for Cycle<S, T>
where
    S: Stream<Item = T>,
    T: Clone,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        let mut next;
        if *this.state == CycleState::FromStream {
            next = futures_core::ready!(this.source.poll_next(cx));

            if let Some(val) = next {
                this.buffer.push(val.clone());
                next = Some(val)
            } else {
                *this.state = CycleState::FromBuffer;
                next = this.buffer.get(*this.index).cloned();
            }
        } else {
            let mut index = *this.index;
            if index == this.buffer.len() {
                index = 0
            }
            next = Some(this.buffer[index].clone());

            *this.index = index + 1;
        }

        Poll::Ready(next)
    }
}
