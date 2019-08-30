use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::ready;
use futures::stream::Stream;

/// Creates a new stream where each iteration calls the provided closure.
///
/// This allows creating a custom iterator with any behavior
/// without using the more verbose syntax of creating a dedicated type
/// and implementing the `Iterator` trait for it.
#[inline]
pub fn from_fn<T, F>(f: F) -> FromFn<F, T>
    where F: FnMut() -> Box<dyn Future<Output = Option<T>>>,
          F: Unpin
{
    FromFn{
        closure: f,
        fut: None,
    }
}

/// A stream where each iteration calls the provided closure.
///
/// This `struct` is created by the [`stream::from_fn`] function.
/// See its documentation for more.
///
/// [`stream::from_fn`]: fn.from_fn.html
pub struct FromFn<F, T> {
    closure: F,
    fut: Option<Box<dyn Future<Output = Option<T>>>>,
}

impl<T, F: Unpin> Stream for FromFn<F, T>
    where F: FnMut() -> Box<dyn Future<Output = Option<T>>>
{
    type Item = T;

    #[inline]
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let None = self.fut {
            self.fut = Some((self.closure)());
        }
        
        let pinned = unsafe { Pin::new_unchecked(&mut self.fut.as_mut().unwrap()) };
        let out = ready!(pinned.poll(cx));

        self.fut = None;
        Poll::Ready(out)
    }
}

impl<F, T> fmt::Debug for FromFn<F, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FromFn").finish()
    }
}
