use crate::stream::Stream;

use std::pin::Pin;
use std::task::{Context, Poll};
use crate::stream::DoubleEndedStream;

/// A double-ended stream that was created from iterator.
///
/// This stream is created by the [`from_iter`] function.
/// See it documentation for more.
///
/// [`from_iter`]: fn.from_iter.html
#[derive(Debug)]
pub struct FromIter<T> {
    inner: Vec<T>,
}

pub fn from_iter<I: IntoIterator>(iter: I) -> FromIter<I::Item> {
    FromIter { inner: iter.into_iter().collect() }
}

impl<T> Unpin for FromIter<T> {}

impl<T> Stream for FromIter<T> {
    type Item = T;
    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.inner.len() > 0 {
            return Poll::Ready(Some(self.inner.remove(0)));
        }
        return Poll::Ready(None);
    }
}

impl<T> DoubleEndedStream for FromIter<T> {
    fn poll_next_back(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.inner.pop())
    }
}
