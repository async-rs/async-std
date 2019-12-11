use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::stream::{Stream, DoubleEndedStream, ExactSizeStream, FusedStream};

/// A stream that never returns any items.
/// 
/// This stream is created by the [`pending`] function. See its
/// documentation for more.
/// 
/// [`pending`]: fn.pending.html
#[derive(Debug)]
pub struct Pending<T> {
    _marker: PhantomData<T>
}

/// Creates a stream that never returns any items.
/// 
/// The returned stream will always return `Pending` when polled.
pub fn pending<T>() -> Pending<T> {
    Pending { _marker: PhantomData }
}

impl<T> Stream for Pending<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<T>> {
        Poll::Pending
    }
}

impl<T> DoubleEndedStream for Pending<T> {
    fn poll_next_back(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<T>> {
        Poll::Pending
    }
}

impl<T> FusedStream for Pending<T> {}

impl<T> ExactSizeStream for Pending<T> {
    fn len(&self) -> usize {
        0
    }
}
