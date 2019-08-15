use std::pin::Pin;
use std::sync::Mutex;

use crate::task::{Context, Poll};

/// Creates a stream that yields the given elements continually.
///
/// # Examples
///
/// ```
/// # #![feature(async_await)]
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::prelude::*;
/// use async_std::stream;
///
/// let mut s = stream::cycle(vec![1, 2, 3]);
///
/// assert_eq!(s.next().await, Some(1));
/// assert_eq!(s.next().await, Some(2));
/// assert_eq!(s.next().await, Some(3));
/// assert_eq!(s.next().await, Some(1));
/// assert_eq!(s.next().await, Some(2));
/// #
/// # }) }
/// ```
pub fn cycle<T>(items: Vec<T>) -> Cycle<T>
where
    T: Clone,
{
    Cycle {
        items,
        cursor: Mutex::new(0_usize),
    }
}

/// A stream that yields the given elements continually.
///
/// This stream is constructed by the [`cycle`] function.
///
/// [`cycle`]: fn.cycle.html
#[derive(Debug)]
pub struct Cycle<T> {
    items: Vec<T>,
    cursor: Mutex<usize>,
}

impl<T: Clone> futures::Stream for Cycle<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let cursor = &mut *self.cursor.lock().unwrap();
        let p = Poll::Ready(self.items.get(*cursor).map(|x| x.to_owned()));
        *cursor = (*cursor + 1_usize) % self.items.len();
        p
    }
}
