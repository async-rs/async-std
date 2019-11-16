use crate::stream::Stream;

use std::pin::Pin;
use std::task::{Context, Poll};

/// A stream able to yield elements from both ends.
///
/// Something that implements `DoubleEndedStream` has one extra capability
/// over something that implements [`Stream`]: the ability to also take
/// `Item`s from the back, as well as the front.
///
/// [`Stream`]: trait.Stream.html
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
pub trait DoubleEndedStream: Stream {
    /// Removes and returns an element from the end of the stream.
    ///
    /// Returns `None` when there are no more elements.
    ///
    /// The [trait-level] docs contain more details.
    ///
    /// [trait-level]: trait.DoubleEndedStream.html
    fn poll_next_back(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
}

pub struct Sample<T> {
    inner: Vec<T>,
}

impl<T> From<Vec<T>> for Sample<T> {
    fn from(data: Vec<T>) -> Self {
        Sample { inner: data }
    }
}

impl<T> Unpin for Sample<T> {}

impl<T> Stream for Sample<T> {
    type Item = T;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.inner.len() > 0 {
            return Poll::Ready(Some(self.inner.remove(0)));
        }
        return Poll::Ready(None);
    }
}

impl<T> DoubleEndedStream for Sample<T> {
    fn poll_next_back(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.inner.pop())
    }
}
