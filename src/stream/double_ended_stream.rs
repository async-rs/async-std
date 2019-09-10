use crate::stream::Stream;

use std::pin::Pin;
use std::task::{Context, Poll};

/// An stream able to yield elements from both ends.
///
/// Something that implements `DoubleEndedStream` has one extra capability
/// over something that implements [`Stream`]: the ability to also take
/// `Item`s from the back, as well as the front.
///
/// [`Stream`]: trait.Stream.html
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
