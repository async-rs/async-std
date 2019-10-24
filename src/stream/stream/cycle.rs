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

/// # Examples
///
/// Basic usage:
///
/// ```
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
///
///  let values = vec![1,2,3];
///
/// # Ok(()) }) }
///```
fn cycle<T: Copy>(values: Vec<T>) -> impl Stream<Item = T> {
    Cycle {
        source: values,
        index: 0,
    }
}
