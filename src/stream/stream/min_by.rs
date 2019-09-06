use std::cmp::Ordering;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

/// A future that yields the minimum item in a stream by a given comparison function.
#[derive(Clone, Debug)]
pub struct MinByFuture<S: Stream, F> {
    stream: S,
    compare: F,
    min: Option<S::Item>,
}

impl<S: Stream + Unpin, F> Unpin for MinByFuture<S, F> {}

impl<S: Stream + Unpin, F> MinByFuture<S, F> {
    pub(super) fn new(stream: S, compare: F) -> Self {
        MinByFuture {
            stream,
            compare,
            min: None,
        }
    }
}

impl<S, F> Future for MinByFuture<S, F>
where
    S: futures_core::stream::Stream + Unpin,
    S::Item: Copy,
    F: FnMut(&S::Item, &S::Item) -> Ordering,
{
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let next = futures_core::ready!(Pin::new(&mut self.stream).poll_next(cx));

        match next {
            Some(new) => {
                cx.waker().wake_by_ref();
                match self.as_mut().min.take() {
                    None => self.as_mut().min = Some(new),
                    Some(old) => match (&mut self.as_mut().compare)(&new, &old) {
                        Ordering::Less => self.as_mut().min = Some(new),
                        _ => self.as_mut().min = Some(old),
                    },
                }
                Poll::Pending
            }
            None => Poll::Ready(self.min),
        }
    }
}
