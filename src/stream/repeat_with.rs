use std::marker::PhantomData;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A stream that repeats elements of type `T` endlessly by applying a provided closure.
///
/// This stream is constructed by the [`repeat_with`] function.
///
/// [`repeat_with`]: fn.repeat_with.html
#[derive(Debug)]
pub struct RepeatWith<F, Fut, A> {
    f: F,
    future: Option<Fut>,
    __a: PhantomData<A>,
}

/// Creates a new stream that repeats elements of type `A` endlessly by applying the provided closure.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::prelude::*;
/// use async_std::stream;
///
/// let s = stream::repeat_with(|| async { 1 });
///
/// pin_utils::pin_mut!(s);
///
/// assert_eq!(s.next().await, Some(1));
/// assert_eq!(s.next().await, Some(1));
/// assert_eq!(s.next().await, Some(1));
/// assert_eq!(s.next().await, Some(1));
/// # }) }
/// ```
///
/// Going finite:
///
/// ```
/// # fn main() { async_std::task::block_on(async {
/// #
/// use async_std::prelude::*;
/// use async_std::stream;
///
/// let s = stream::repeat_with(|| async { 1u8 }).take(2);
///
/// pin_utils::pin_mut!(s);
///
/// assert_eq!(s.next().await, Some(1));
/// assert_eq!(s.next().await, Some(1));
/// assert_eq!(s.next().await, None);
/// # }) }
/// ```
pub fn repeat_with<F, Fut, A>(repeater: F) -> RepeatWith<F, Fut, A>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = A>,
{
    RepeatWith {
        f: repeater,
        future: None,
        __a: PhantomData,
    }
}

impl<F, Fut, A> RepeatWith<F, Fut, A> {
    pin_utils::unsafe_unpinned!(f: F);
    pin_utils::unsafe_pinned!(future: Option<Fut>);
}

impl<F, Fut, A> Stream for RepeatWith<F, Fut, A>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = A>,
{
    type Item = A;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match &self.future {
                Some(_) => {
                    let res =
                        futures_core::ready!(self.as_mut().future().as_pin_mut().unwrap().poll(cx));

                    self.as_mut().future().set(None);

                    return Poll::Ready(Some(res));
                }
                None => {
                    let fut = (self.as_mut().f())();

                    self.as_mut().future().set(Some(fut));
                }
            }
        }
    }
}
