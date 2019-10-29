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
                next = Some(val.clone())
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

/// Creats a stream that yields the provided values infinitely and in order.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use async_std::prelude::*;
/// use async_std::stream;
///
/// let mut s = stream::cycle(stream::once(7));
///
/// assert_eq!(s.next().await, Some(7));
/// assert_eq!(s.next().await, Some(7));
/// assert_eq!(s.next().await, Some(7));
/// assert_eq!(s.next().await, Some(7));
/// assert_eq!(s.next().await, Some(7));
/// #
/// # })
/// ```
pub fn cycle<S: Stream<Item = T>, T: Clone>(source: S) -> impl Stream<Item = S::Item> {
    Cycle {
        source,
        index: 0,
        buffer: Vec::new(),
        state: CycleState::FromStream,
    }
}
