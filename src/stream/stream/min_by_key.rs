use std::cmp::Ordering;
use std::pin::Pin;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct MinByKeyFuture<S, T, K> {
    stream: S,
    min: Option<T>,
    key_by: K,
}

impl<S, T, K> MinByKeyFuture<S, T, K> {
    pin_utils::unsafe_pinned!(stream: S);
    pin_utils::unsafe_unpinned!(min: Option<T>);
    pin_utils::unsafe_unpinned!(key_by: K);

    pub(super) fn new(stream: S, key_by: K) -> Self {
        MinByKeyFuture {
            stream,
            min: None,
            key_by,
        }
    }
}

impl<S, K> Future for MinByKeyFuture<S, S::Item, K>
where
    S: Stream + Unpin + Sized,
    K: FnMut(&S::Item) -> S::Item,
    S::Item: Ord + Copy,
{
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let next = futures_core::ready!(self.as_mut().stream().poll_next(cx));

        match next {
            Some(new) => {
                let new = self.as_mut().key_by()(&new);
                cx.waker().wake_by_ref();
                match self.as_mut().min().take() {
                    None => *self.as_mut().min() = Some(new),

                    Some(old) => match new.cmp(&old) {
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
