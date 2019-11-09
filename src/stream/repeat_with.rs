use std::marker::PhantomData;
use std::pin::Pin;
use std::future::Future;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A stream that repeats elements of type `T` endlessly by applying a provided closure.
    ///
    /// This stream is created by the [`repeat_with`] function. See its
    /// documentation for more.
    ///
    /// [`repeat_with`]: fn.repeat_with.html
    #[derive(Debug)]
    pub struct RepeatWith<F, Fut, A> {
        f: F,
        #[pin]
        future: Option<Fut>,
        __a: PhantomData<A>,
    }
}

/// Creates a new stream that repeats elements of type `A` endlessly by applying the provided closure.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # async_std::task::block_on(async {
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
/// # })
/// ```
///
/// Going finite:
///
/// ```
/// # async_std::task::block_on(async {
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
/// # })
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

impl<F, Fut, A> Stream for RepeatWith<F, Fut, A>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = A>,
{
    type Item = A;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            if this.future.is_some() {
                let res = futures_core::ready!(this.future.as_mut().as_pin_mut().unwrap().poll(cx));

                this.future.set(None);

                return Poll::Ready(Some(res));
            } else {
                let fut = (this.f)();

                this.future.set(Some(fut));
            }
        }
    }
}
