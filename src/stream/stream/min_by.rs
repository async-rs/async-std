use std::cmp::Ordering;
use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct MinMaxByFuture<S, F, T> {
        #[pin]
        stream: S,
        compare: F,
        min: Option<T>,
        direction: Direction,
    }
}
enum Direction {
    Maximizing,
    Minimizing,
}


impl<S, F, T> MinMaxByFuture<S, F, T> {
    pub(super) fn new(stream: S, compare: F) -> Self {
        MinMaxByFuture {
            stream,
            compare,
            min: None,
            direction: Direction::Minimizing,
        }
    }
}

impl<S, F> Future for MinMaxByFuture<S, F, S::Item>
where
    S: Stream + Unpin + Sized,
    S::Item: Copy,
    F: FnMut(&S::Item, &S::Item) -> Ordering,
{
    type Output = Option<S::Item>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let next = futures_core::ready!(this.stream.poll_next(cx));

        match next {
            Some(new) => {
                cx.waker().wake_by_ref();
                match this.min.take() {
                    None => *this.min = Some(new),
                    Some(old) => match (this.compare)(&new, &old) {
                        Ordering::Less => *this.min = Some(new),
                        _ => *this.min = Some(old),
                    },
                }
                Poll::Pending
            }
            None => Poll::Ready(*this.min),
        }
    }
}
