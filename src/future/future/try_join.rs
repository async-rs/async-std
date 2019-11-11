use std::pin::Pin;

use async_macros::MaybeDone;
use pin_project_lite::pin_project;

use crate::task::{Context, Poll};
use std::future::Future;

pin_project! {
    #[allow(missing_docs)]
    #[allow(missing_debug_implementations)]
    pub struct TryJoin<L, R>
    where
        L: Future,
        R: Future<Output = L::Output>
    {
        #[pin] left: MaybeDone<L>,
        #[pin] right: MaybeDone<R>,
    }
}

impl<L, R> TryJoin<L, R>
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

impl<L, R, T, E> Future for TryJoin<L, R>
where
    L: Future<Output = Result<T, E>>,
    R: Future<Output = L::Output>,
{
    type Output = Result<(T, T), E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        let mut left = this.left;
        let mut right = this.right;

        if Future::poll(Pin::new(&mut left), cx).is_ready() {
            if left.as_ref().output().unwrap().is_err() {
                return Poll::Ready(Err(left.take().unwrap().err().unwrap()));
            } else if right.as_ref().output().is_some() {
                return Poll::Ready(Ok((
                    left.take().unwrap().ok().unwrap(),
                    right.take().unwrap().ok().unwrap(),
                )));
            }
        }

        if Future::poll(Pin::new(&mut right), cx).is_ready() {
            if right.as_ref().output().unwrap().is_err() {
                return Poll::Ready(Err(right.take().unwrap().err().unwrap()));
            } else if left.as_ref().output().is_some() {
                return Poll::Ready(Ok((
                    left.take().unwrap().ok().unwrap(),
                    right.take().unwrap().ok().unwrap(),
                )));
            }
        }

        Poll::Pending
    }
}
