use crate::stream::Stream;

use std::pin::Pin;
use std::task::{Context, Poll};
use crate::stream::DoubleEndedStream;

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
