use std::marker::PhantomData;
use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A stream that yields elements by calling a closure.
    ///
    /// This stream is constructed by [`from_fn`] function.
    ///
    /// [`from_fn`]: fn.from_fn.html
    #[derive(Debug)]
    pub struct FromFn<F, Fut, T> {
        f: F,
        #[pin]
        future: Option<Fut>,
        __t: PhantomData<T>,
    }
}

/// Creates a new stream where to produce each new element a provided closure is called.
///
/// This allows creating a custom stream with any behaviour without using the more verbose
/// syntax of creating a dedicated type and implementing a `Stream` trait for it.
///
/// # Examples
///
/// ```
/// # async_std::task::block_on(async {
/// #
/// use async_std::prelude::*;
/// use async_std::sync::Mutex;
/// use std::sync::Arc;
/// use async_std::stream;
///
/// let count = Arc::new(Mutex::new(0u8));
/// let s = stream::from_fn(|| {
///     let count = Arc::clone(&count);
///
///     async move {
///         *count.lock().await += 1;
///
///         if *count.lock().await > 3 {
///             None
///         } else {
///             Some(*count.lock().await)
///         }
///     }
/// });
///
/// pin_utils::pin_mut!(s);
/// assert_eq!(s.next().await, Some(1));
/// assert_eq!(s.next().await, Some(2));
/// assert_eq!(s.next().await, Some(3));
/// assert_eq!(s.next().await, None);
/// #
/// # })
/// ```
pub fn from_fn<T, F, Fut>(f: F) -> FromFn<F, Fut, T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<T>>,
{
    FromFn {
        f,
        future: None,
        __t: PhantomData,
    }
}

impl<F, Fut, T> Stream for FromFn<F, Fut, T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<T>>,
{
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            if this.future.is_some() {
                let next =
                    futures_core::ready!(this.future.as_mut().as_pin_mut().unwrap().poll(cx));
                this.future.set(None);

                return Poll::Ready(next);
            } else {
                let fut = (this.f)();
                this.future.set(Some(fut));
            }
        }
    }
}
