use std::pin::Pin;
use std::marker::PhantomData;

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
pub struct Successor<F, Fut, T> 
where Fut: Future<Output=T>
{
    successor: F,
    future: Option<Fut>,
    next: T,
    _marker: PhantomData<Fut>
}

/// Creates a new stream where to produce each new element a clousre is called with the previous
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
/// let s = stream::successor(22, |val| {
///     async move {
///         val + 1
///     }
/// });
///
/// pin_utils::pin_mut!(s);
/// assert_eq!(s.next().await, Some(1));
/// assert_eq!(s.next().await, Some(2));
/// assert_eq!(s.next().await, Some(3));
/// #
/// # }) }
///
/// ```
pub fn successor<F, Fut, T>(start: T, func: F) -> Successor<F, Fut, T>
where
    F: FnMut(T) -> Fut,
    Fut: Future<Output = T>,
    T: Copy,
    {
        Successor {
            successor: func,
            future: None,
            next: start,
            _marker: PhantomData,
        }
    }

impl <F, Fut, T> Successor<F, Fut, T>
where
    F: FnMut(T) -> Fut,
    Fut: Future<Output = T>,
    T: Copy,

{
    pin_utils::unsafe_unpinned!(successor: F);
    pin_utils::unsafe_unpinned!(next: T);
    pin_utils::unsafe_pinned!(future: Option<Fut>);

}

impl <F, Fut, T> Stream for Successor<F, Fut, T>
where
    Fut: Future<Output = T>,
    F: FnMut(T) -> Fut,
    T: Copy,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match &self.future {
            Some(_) => {
                let next = futures_core::ready!(self.as_mut().future().as_pin_mut().unwrap().poll(cx));
                self.as_mut().future().set(None);

                Poll::Ready(Some(next))
            },
            None => {
                let x = self.next;
                let fut = (self.as_mut().successor())(x);
                self.as_mut().future().set(Some(fut));
                // Probably can poll the value here?
                Poll::Pending
            }
        }
    }
}
