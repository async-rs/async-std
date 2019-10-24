use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream that will repeatedly yield the same list of elements
pub struct Cycle<T> {
    source: Vec<T>,
    index: usize,
}

impl<T: Copy> Stream for Cycle<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Pending
    }
}
