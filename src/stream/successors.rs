use std::pin::Pin;
use std::mem;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll, ready};

use pin_project_lite::pin_project;

/// Creates a new stream where to produce each new element a closure is called with the previous
/// value.
///
/// # Examples
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
/// assert_eq!(s.next().await, Some(22));
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
#[cfg(feature = "unstable")]
#[cfg_attr(feature = "docs", doc(cfg(unstable)))]
pub fn successors<F, Fut, T>(first: Option<T>, succ: F) -> Successors<F, Fut, T>
where
    F: FnMut(T) -> Fut,
    Fut: Future<Output = Option<T>>,
    T: Copy,
{
    Successors {
        succ: succ,
        future: None,
        slot: first,
    }
}

pin_project! {
    /// A stream that yields elements by calling an async closure with the previous value as an
    /// argument
    ///
    /// This stream is constructed by [`successors`] function
    ///
    /// [`successors`]: fn.succssors.html
    #[cfg(feature = "unstable")]
    #[cfg_attr(feature = "docs", doc(cfg(unstable)))]
    #[derive(Debug)]
    pub struct Successors<F, Fut, T>
    where
        Fut: Future<Output = Option<T>>,
    {
        succ: F,
        #[pin]
        future: Option<Fut>,
        slot: Option<T>,
    }
}

impl<F, Fut, T> Stream for Successors<F, Fut, T>
where
    Fut: Future<Output = Option<T>>,
    F: FnMut(T) -> Fut,
    T: Copy,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if this.slot.is_none() {
            return Poll::Ready(None);
        }

        if this.future.is_none() {
            let fut = (this.succ)(this.slot.unwrap());
            this.future.set(Some(fut));
        }

        let mut next = ready!(this.future.as_mut().as_pin_mut().unwrap().poll(cx));

        this.future.set(None);

        // 'swapping' here means 'slot' will hold the next value and next will be th one from the previous iteration
        mem::swap(this.slot, &mut next);
        Poll::Ready(next)
    }
}
