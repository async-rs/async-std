use std::pin::Pin;

use crate::stream::Stream;
use crate::task::{Context, Poll};

/// Creates a new stream where each successive item is computed based on the preceding one.
/// The stream starts with the given first item (if any) and calls the given FnMut(&T) -> Option<T> closure to compute each item’s successor.
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use async_std::prelude::*;
/// use async_std::stream;
///
/// let powers_of_10 = stream::successors(Some(1_u16), |n| n.checked_mul(10));
/// assert_eq!(powers_of_10.collect::<Vec<_>>().await, &[1, 10, 100, 1_000, 10_000]);
///
/// #
/// # })
/// ```
pub fn successors<T, F>(first: Option<T>, succ: F) -> Successors<T, F>
where
    F: FnMut(&T) -> Option<T>,
{
    Successors { next: first, succ }
}

/// An new stream where each successive item is computed based on the preceding one.
///
/// This `struct` is created by the [`successors`] function.
/// See its documentation for more.
///
/// [`successors`]: fn.successors.html
#[derive(Clone, Debug)]
pub struct Successors<T, F> {
    next: Option<T>,
    succ: F,
}

impl<T, F> Successors<T, F> {
    pin_utils::unsafe_unpinned!(next: Option<T>);
    pin_utils::unsafe_unpinned!(succ: F);
}

impl<T, F> Stream for Successors<T, F>
where
    F: FnMut(&T) -> Option<T>,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let item = self.as_mut().next().take().map(|item| {
            *self.as_mut().next() = (self.as_mut().succ())(&item);
            item
        });

        Poll::Ready(item)
    }
}
