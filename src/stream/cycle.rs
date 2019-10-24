use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A stream that will repeatedly yield the same list of elements
    pub struct Cycle<T> {
        source: Vec<T>,
        index: usize,
        len: usize,
    }
}

impl<T: Copy> Stream for Cycle<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let value = self.source[self.index];

        let next = self.index + 1;

        self.as_mut().index = next % self.len;

        Poll::Ready(Some(value))
    }
}

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
/// let mut s = stream::cycle(vec![1,2,3]);
///
/// assert_eq!(s.next().await, Some(1));
/// assert_eq!(s.next().await, Some(2));
/// assert_eq!(s.next().await, Some(3));
/// assert_eq!(s.next().await, Some(1));
/// assert_eq!(s.next().await, Some(2));
/// #
/// # })
/// ```
pub fn cycle<T: Copy>(source: Vec<T>) -> impl Stream<Item = T> {
    let len = source.len();
    Cycle {
        source,
        index: 0,
        len,
    }
}
