use core::cmp::{Ord, Ordering};
use core::marker::PhantomData;
use core::pin::Pin;
use core::future::Future;

use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct MaxFuture<S, F, T> {
        #[pin]
        stream: S,
        _compare: PhantomData<F>,
        max: Option<T>,
    }
}

impl<S, F, T> MaxFuture<S, F, T> {
    pub(super) fn new(stream: S) -> Self {
        Self {
            stream,
            _compare: PhantomData,
            max: None,
        }
    }
}

impl<S, F> Future for MaxFuture<S, F, S::Item>
where
    S: Stream,
    S::Item: Ord,
    F: FnMut(&S::Item, &S::Item) -> Ordering,
{
    type Output = Option<S::Item>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let next = futures_core::ready!(this.stream.poll_next(cx));

        match next {
            Some(new) => {
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
