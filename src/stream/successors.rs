use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream that yields elements by calling an async closure with the previous value as an
/// argument
///
/// This stream is constructed by [`successor`] function
///
/// [`successor`]: fn.successor.html
#[derive(Debug)]
pub struct Successors<F, Fut, T>
where
    Fut: Future<Output = Option<T>>,
{
    successor: F,
    future: Option<Fut>,
    next: Option<T>,
    _marker: PhantomData<Fut>,
}

/// Creates a new stream where to produce each new element a closure is called with the previous
/// value.
///
/// #Examples
///
/// ```
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::prelude::*;
/// use async_std::stream;
///
/// let s = stream::successors(Some(22), |val| {
///     async move {
///         Some(val + 1)
///     }
/// });
///
/// pin_utils::pin_mut!(s);
/// assert_eq!(s.next().await, Some(23));
/// assert_eq!(s.next().await, Some(24));
/// assert_eq!(s.next().await, Some(25));
///
///
///let never = stream::successors(None, |val: usize| {
///     async move {
///         Some(val + 1)
///     }
/// });
///
/// pin_utils::pin_mut!(never);
/// assert_eq!(never.next().await, None);
/// assert_eq!(never.next().await, None);
/// #
/// # }) }
///
/// ```
pub fn successors<F, Fut, T>(start: Option<T>, func: F) -> Successors<F, Fut, T>
where
    F: FnMut(T) -> Fut,
    Fut: Future<Output = Option<T>>,
    T: Copy,
{
    Successors {
        successor: func,
        future: None,
        next: start,
        _marker: PhantomData,
    }
}

impl<F, Fut, T> Successors<F, Fut, T>
where
    F: FnMut(T) -> Fut,
    Fut: Future<Output = Option<T>>,
    T: Copy,
{
    pin_utils::unsafe_unpinned!(successor: F);
    pin_utils::unsafe_unpinned!(next: Option<T>);
    pin_utils::unsafe_pinned!(future: Option<Fut>);
}

impl<F, Fut, T> Stream for Successors<F, Fut, T>
where
    Fut: Future<Output = Option<T>>,
    F: FnMut(T) -> Fut,
    T: Copy,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.next.is_none() {
            return Poll::Ready(None);
        }

        match &self.future {
            None => {
                let x = self.next.unwrap();
                let fut = (self.as_mut().successor())(x);
                self.as_mut().future().set(Some(fut));
            }
            _ => {}
        }

        let next = futures_core::ready!(self.as_mut().future().as_pin_mut().unwrap().poll(cx));
        *self.as_mut().next() = next;
        self.as_mut().future().set(None);
        Poll::Ready(next)
    }
}
