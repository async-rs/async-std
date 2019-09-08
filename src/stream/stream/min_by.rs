use std::cmp::Ordering;
use std::pin::Pin;

use crate::future::Future;
use crate::task::{Context, Poll};

/// A future that yields the minimum item in a stream by a given comparison function.
#[derive(Debug)]
pub struct MinByFuture<S, F, T> {
    stream: S,
    compare: F,
    min: Option<T>,
}

impl<S, F, T> MinByFuture<S, F, T> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(compare: F);
    pin_utils::unsafe_unpinned!(min: Option<T>);

    pub(super) fn new(stream: S, compare: F) -> Self {
        MinByFuture {
            stream,
            compare,
            min: None,
        }
    }
}

impl<S, F> Future for MinByFuture<S, F, S::Item>
where
    S: futures_core::stream::Stream + Unpin + Sized,
    S::Item: Copy,
    F: FnMut(&S::Item, &S::Item) -> Ordering,
{
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match next {
            Some(new) => {
                cx.waker().wake_by_ref();
                match self.as_mut().min().take() {
                    None => *self.as_mut().min() = Some(new),
                    Some(old) => match (&mut self.as_mut().compare())(&new, &old) {
                        Ordering::Less => *self.as_mut().min() = Some(new),
                        _ => *self.as_mut().min() = Some(old),
                    },
                }
                Poll::Pending
            }
            None => Poll::Ready(self.min),
        }
    }
}
