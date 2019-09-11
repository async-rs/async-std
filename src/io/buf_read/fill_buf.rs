use std::pin::Pin;

use futures_io::AsyncBufRead;

use crate::future::Future;
use crate::io;
use crate::task::{Context, Poll};

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FillBufFuture<'a, R: ?Sized> {
    reader: &'a mut R,
}

impl<'a, R: ?Sized> FillBufFuture<'a, R> {
    pub(crate) fn new(reader: &'a mut R) -> Self {
        Self { reader }
    }
}

impl<'a, R: AsyncBufRead + Unpin + ?Sized> Future for FillBufFuture<'a, R> {
    type Output = io::Result<&'a [u8]>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&'a [u8]>> {
        let Self { reader } = &mut *self;
        let result = Pin::new(reader).poll_fill_buf(cx);
        // This is safe because:
        // 1. The buffer is valid for the lifetime of the reader.
        // 2. Output is unrelated to the wrapper (Self).
        result.map_ok(|buf| unsafe { std::mem::transmute::<&'_ [u8], &'a [u8]>(buf) })
    }
}
