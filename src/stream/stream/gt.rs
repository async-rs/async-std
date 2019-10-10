use std::cmp::Ordering;
use std::pin::Pin;

use super::partial_cmp::PartialCmpFuture;
use crate::future::Future;
use crate::prelude::*;
use crate::stream::Stream;
use crate::task::{Context, Poll};

// Determines if the elements of this `Stream` are lexicographically
// greater than or equal to those of another.
#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct GtFuture<L: Stream, R: Stream> {
    partial_cmp: PartialCmpFuture<L, R>,
}

impl<L: Stream, R: Stream> GtFuture<L, R>
where
    L::Item: PartialOrd<R::Item>,
{
    pin_utils::unsafe_pinned!(partial_cmp: PartialCmpFuture<L, R>);

    pub(super) fn new(l: L, r: R) -> Self {
        GtFuture {
            partial_cmp: l.partial_cmp(r),
        }
    }
}

impl<L: Stream, R: Stream> Future for GtFuture<L, R>
where
    L: Stream + Sized,
    R: Stream + Sized,
    L::Item: PartialOrd<R::Item>,
{
    type Output = bool;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let result = futures_core::ready!(self.as_mut().partial_cmp().poll(cx));

        match result {
            Some(Ordering::Greater) => Poll::Ready(true),
            _ => Poll::Ready(false),
        }
    }
}