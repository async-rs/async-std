use std::pin::Pin;

use super::fuse::Fuse;
use crate::prelude::*;
use crate::task::{Context, Poll};

/// Chains two streams one after another.
#[derive(Debug)]
pub struct Chain<S, U> {
    first: Fuse<S>,
    second: Fuse<U>,
}

impl<S: Stream, U: Stream> Chain<S, U> {
    pin_utils::unsafe_pinned!(first: Fuse<S>);
    pin_utils::unsafe_pinned!(second: Fuse<U>);

    pub(super) fn new(first: S, second: U) -> Self {
        Chain {
            first: first.fuse(),
            second: second.fuse(),
        }
    }
}

impl<S: Stream, U: Stream<Item = S::Item>> Stream for Chain<S, U> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if !self.first.done {
            let next = futures_core::ready!(self.as_mut().first().poll_next(cx));
            if let Some(next) = next {
                return Poll::Ready(Some(next));
            }
        }

        if !self.second.done {
            let next = futures_core::ready!(self.as_mut().second().poll_next(cx));
            if let Some(next) = next {
                return Poll::Ready(Some(next));
            }
        }

        if self.first.done && self.second.done {
            return Poll::Ready(None);
        }

        Poll::Pending
    }
}
