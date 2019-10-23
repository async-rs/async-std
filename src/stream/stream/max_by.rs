use std::cmp::Ordering;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct MaxByFuture<S, F, T> {
    stream: S,
    compare: F,
    max: Option<T>,
}

impl<S, F, T> MaxByFuture<S, F, T> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(compare: F);
    pin_utils::unsafe_unpinned!(max: Option<T>);

    pub(super) fn new(stream: S, compare: F) -> Self {
        MaxByFuture {
            stream,
            compare,
            max: None,
        }
    }
}

impl<S, F> Future for MaxByFuture<S, F, S::Item>
where
    S: Stream + Unpin + Sized,
    S::Item: Copy,
    F: FnMut(&S::Item, &S::Item) -> Ordering,
{
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match next {
            Some(new) => {
                cx.waker().wake_by_ref();
                match self.as_mut().max().take() {
                    None => *self.as_mut().max() = Some(new),
                    Some(old) => match (&mut self.as_mut().compare())(&new, &old) {
                        Ordering::Greater => *self.as_mut().max() = Some(new),
                        _ => *self.as_mut().max() = Some(old),
                    },
                }
                Poll::Pending
            }
            None => Poll::Ready(self.max),
        }
    }
}
