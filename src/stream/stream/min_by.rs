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
        value: Option<T>,
        direction: Direction,
    }
}

#[derive(PartialEq, Eq)]
enum Direction {
    Maximizing,
    Minimizing,
}


impl<S, F, T> MinMaxByFuture<S, F, T> {
    pub(super) fn new_min(stream: S, compare: F) -> Self {
        MinMaxByFuture {
            stream,
            compare,
            value: None,
            direction: Direction::Minimizing,
        }
    }

    pub(super) fn new_max(stream: S, compare: F) -> Self {
        MinMaxByFuture {
            stream,
            compare,
            value: None,
            direction: Direction::Maximizing,
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

                match this.value.take() {
                    None => this.value.replace(new),
                    Some(old) => match (this.compare)(&new, &old) {
                        Ordering::Less if Direction::Minimizing == *this.direction => this.value.replace(new),
                        Ordering::Greater if Direction::Maximizing == *this.direction => this.value.replace(new),
                        _ => this.value.replace(old),
                    },
                };
                Poll::Pending
            }
            None => Poll::Ready(*this.value),
        }
    }
}
