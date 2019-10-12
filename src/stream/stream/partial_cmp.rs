use std::cmp::Ordering;
use std::pin::Pin;

use super::fuse::Fuse;
use crate::future::Future;
use crate::prelude::*;
use crate::stream::Stream;
use crate::task::{Context, Poll};

// Lexicographically compares the elements of this `Stream` with those
// of another.
#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct PartialCmpFuture<L: Stream, R: Stream> {
    l: Fuse<L>,
    r: Fuse<R>,
    l_cache: Option<L::Item>,
    r_cache: Option<R::Item>,
}

impl<L: Stream, R: Stream> PartialCmpFuture<L, R> {
    pin_utils::unsafe_pinned!(l: Fuse<L>);
    pin_utils::unsafe_pinned!(r: Fuse<R>);
    pin_utils::unsafe_unpinned!(l_cache: Option<L::Item>);
    pin_utils::unsafe_unpinned!(r_cache: Option<R::Item>);

    pub(super) fn new(l: L, r: R) -> Self {
        PartialCmpFuture {
            l: l.fuse(),
            r: r.fuse(),
            l_cache: None,
            r_cache: None,
        }
    }
}

impl<L: Stream, R: Stream> Future for PartialCmpFuture<L, R>
where
    L: Stream + Sized,
    R: Stream + Sized,
    L::Item: PartialOrd<R::Item>,
{
    type Output = Option<Ordering>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            // Short circuit logic
            // Stream that completes earliest can be considered Less, etc
            let l_complete = self.l.done && self.as_mut().l_cache.is_none();
            let r_complete = self.r.done && self.as_mut().r_cache.is_none();

            if l_complete && r_complete {
                return Poll::Ready(Some(Ordering::Equal));
            } else if l_complete {
                return Poll::Ready(Some(Ordering::Less));
            } else if r_complete {
                return Poll::Ready(Some(Ordering::Greater));
            }

            // Get next value if possible and necesary
            if !self.l.done && self.as_mut().l_cache.is_none() {
                let l_next = futures_core::ready!(self.as_mut().l().poll_next(cx));
                if let Some(item) = l_next {
                    *self.as_mut().l_cache() = Some(item);
                }
            }

            if !self.r.done && self.as_mut().r_cache.is_none() {
                let r_next = futures_core::ready!(self.as_mut().r().poll_next(cx));
                if let Some(item) = r_next {
                    *self.as_mut().r_cache() = Some(item);
                }
            }

            // Compare if both values are available.
            if self.as_mut().l_cache.is_some() && self.as_mut().r_cache.is_some() {
                let l_value = self.as_mut().l_cache().take().unwrap();
                let r_value = self.as_mut().r_cache().take().unwrap();
                let result = l_value.partial_cmp(&r_value);

                if let Some(Ordering::Equal) = result {
                    // Reset cache to prepare for next comparison
                    *self.as_mut().l_cache() = None;
                    *self.as_mut().r_cache() = None;
                } else {
                    // Return non equal value
                    return Poll::Ready(result);
                }
            }
        }
    }
}