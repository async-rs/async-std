use std::pin::Pin;

use pin_project_lite::pin_project;
use async_macros::MaybeDone;

use crate::future::Future;
// use crate::future::MaybeDone;
use crate::task::{Context, Poll};

pin_project! {
    #[allow(missing_docs)]
    #[allow(missing_debug_implementations)]
    pub struct Select<L, R> where L: Future, R: Future<Output = L::Output> {
        #[pin] left: MaybeDone<L>,
        #[pin] right: MaybeDone<R>,
    }
}

impl<L, R> Select<L, R>
where
    L: Future,
    R: Future<Output = L::Output>,
{
    pub(crate) fn new(left: L, right: R) -> Self {
        Self {
            left: MaybeDone::new(left),
            right: MaybeDone::new(right),
        }
    }
}

impl<L, R> Future for Select<L, R>
where
    L: Future,
    R: Future<Output = L::Output>,
{
    type Output = L::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let mut left = this.left;
        if Future::poll(Pin::new(&mut left), cx).is_ready() {
            return Poll::Ready(left.take().unwrap());
        }

        let mut right = this.right;
        if Future::poll(Pin::new(&mut right), cx).is_ready() {
            return Poll::Ready(right.take().unwrap());
        }

        Poll::Pending
    }
}
