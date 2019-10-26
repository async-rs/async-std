use std::cmp::Ordering;
use std::pin::Pin;

use pin_project_lite::pin_project;

use crate::future::Future;
use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct MinByKeyFuture<S, T, K> {
        #[pin]
        stream: S,
        min: Option<T>,
        key_by: K,
    }
}

impl<S, T, K> MinByKeyFuture<S, T, K> {
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

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let next = futures_core::ready!(this.stream.poll_next(cx));

        match next {
            Some(new) => {
                let new = (this.key_by)(&new);
                cx.waker().wake_by_ref();
                match this.min.take() {
                    None => *this.min = Some(new),

                    Some(old) => match new.cmp(&old) {
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
