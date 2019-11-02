use std::cmp::Ordering;
use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct MaxByKeyFuture<S, T, K> {
        #[pin]
        stream: S,
        max: Option<T>,
        key_by: K,
    }
}

impl<S, T, K> MaxByKeyFuture<S, T, K> {
    pub(super) fn new(stream: S, key_by: K) -> Self {
        Self {
            stream,
            max: None,
            key_by,
        }
    }
}

impl<S, K> Future for MaxByKeyFuture<S, S::Item, K>
where
    S: Stream,
    K: FnMut(&S::Item) -> S::Item,
    S::Item: Ord,
{
    type Output = Option<S::Item>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let next = futures_core::ready!(this.stream.poll_next(cx));

        match next {
            Some(new) => {
                let new = (this.key_by)(&new);
                cx.waker().wake_by_ref();
                match this.max.take() {
                    None => *this.max = Some(new),

                    Some(old) => match new.cmp(&old) {
                        Ordering::Greater => *this.max = Some(new),
                        _ => *this.max = Some(old),
                    },
                }
                Poll::Pending
            }
            None => Poll::Ready(this.max.take()),
        }
    }
}
