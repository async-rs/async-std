use std::cmp::Ordering;
use std::pin::Pin;
use std::future::Future;

use pin_project_lite::pin_project;

use super::partial_cmp::PartialCmpFuture;
use crate::prelude::*;
use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    // Determines if the elements of this `Stream` are lexicographically
    // less than those of another.
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct LtFuture<L: Stream, R: Stream> {
        #[pin]
        partial_cmp: PartialCmpFuture<L, R>,
    }
}

impl<L: Stream, R: Stream> LtFuture<L, R>
where
    L::Item: PartialOrd<R::Item>,
{
    pub(super) fn new(l: L, r: R) -> Self {
        LtFuture {
            partial_cmp: l.partial_cmp(r),
        }
    }
}

impl<L: Stream, R: Stream> Future for LtFuture<L, R>
where
    L: Stream + Sized,
    R: Stream + Sized,
    L::Item: PartialOrd<R::Item>,
{
    type Output = bool;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = futures_core::ready!(self.project().partial_cmp.poll(cx));

        match result {
            Some(Ordering::Less) => Poll::Ready(true),
            _ => Poll::Ready(false),
        }
    }
}
