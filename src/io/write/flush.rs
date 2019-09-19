use crate::future::Future;
use crate::task::{Context, Poll};

use std::io;
use std::pin::Pin;

use futures_io::AsyncWrite;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FlushFuture<'a, T: Unpin + ?Sized> {
    pub(crate) writer: &'a mut T,
}

impl<T: AsyncWrite + Unpin + ?Sized> Future for FlushFuture<'_, T> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.writer).poll_flush(cx)
    }
}
