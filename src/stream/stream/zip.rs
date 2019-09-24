use std::fmt;
use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// An iterator that iterates two other iterators simultaneously.
pub struct Zip<A: Stream, B> {
    item_slot: Option<A::Item>,
    first: A,
    second: B,
}

impl<A: Stream + fmt::Debug, B: fmt::Debug> fmt::Debug for Zip<A, B> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Zip")
            .field("first", &self.first)
            .field("second", &self.second)
            .finish()
    }
}

impl<A: Stream + Unpin, B: Unpin> Unpin for Zip<A, B> {}

impl<A: Stream, B> Zip<A, B> {
    pub(crate) fn new(first: A, second: B) -> Self {
        Zip {
            item_slot: None,
            first,
            second,
        }
    }

    pin_utils::unsafe_unpinned!(item_slot: Option<A::Item>);
    pin_utils::unsafe_pinned!(first: A);
    pin_utils::unsafe_pinned!(second: B);
}

impl<A: Stream, B: Stream> Stream for Zip<A, B> {
    type Item = (A::Item, B::Item);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.as_mut().item_slot().is_none() {
            match self.as_mut().first().poll_next(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Ready(Some(item)) => *self.as_mut().item_slot() = Some(item),
            }
        }
        let second_item = futures_core::ready!(self.as_mut().second().poll_next(cx));
        let first_item = self.as_mut().item_slot().take().unwrap();
        Poll::Ready(second_item.map(|second_item| (first_item, second_item)))
    }
}
