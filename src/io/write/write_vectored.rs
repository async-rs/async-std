use crate::future::Future;
use crate::task::{Context, Poll};

use std::io;
use std::io::IoSlice;
use std::pin::Pin;

use futures_io::AsyncWrite;

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct WriteVectoredFuture<'a, T: Unpin + ?Sized> {
    pub(crate) writer: &'a mut T,
    pub(crate) bufs: &'a [IoSlice<'a>],
}

impl<T: AsyncWrite + Unpin + ?Sized> Future for WriteVectoredFuture<'_, T> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let bufs = self.bufs;
        Pin::new(&mut *self.writer).poll_write_vectored(cx, bufs)
    }
}
