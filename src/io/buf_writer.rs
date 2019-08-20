use crate::task::{Context, Poll};
use futures::{ready, AsyncWrite, Future, Stream};
use std::io::{self, IntoInnerError};
use std::pin::Pin;
use std::fmt;
use crate::io::Write;

const DEFAULT_CAPACITY: usize = 8 * 1024;


pub struct BufWriter<W: AsyncWrite> {
    inner: Option<W>,
    buf: Vec<u8>,
    panicked: bool,
}

impl<W: AsyncWrite + Unpin> BufWriter<W> {
    pin_utils::unsafe_pinned!(inner: Option<W>);
    pin_utils::unsafe_unpinned!(panicked: bool);

    pub fn new(inner: W) -> BufWriter<W> {
        BufWriter::with_capacity(DEFAULT_CAPACITY, inner)
    }

    pub fn with_capacity(capacity: usize, inner: W) -> BufWriter<W> {
        BufWriter {
            inner: Some(inner),
            buf: Vec::with_capacity(capacity),
            panicked: false,
        }
    }

    pub fn get_ref(&self) -> &W {
        self.inner.as_ref().unwrap()
    }

    pub fn get_mut(&mut self) -> &mut W {
        self.inner.as_mut().unwrap()
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }

    pub fn poll_flush_buf(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let Self {
            inner,
            buf,
            panicked
        } = Pin::get_mut(self);
        let mut panicked = Pin::new(panicked);
        let mut written = 0;
        let len = buf.len();
        let mut ret = Ok(());
        while written < len {
            *panicked = true;
            let r = Pin::new(inner.as_mut().unwrap());
            *panicked = false;
            match r.poll_write(cx, &buf[written..]) {
                Poll::Ready(Ok(0)) => {
                    ret = Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "Failed to write buffered data",
                    ));
                    break;
                }
                Poll::Ready(Ok(n)) => written += n,
                Poll::Ready(Err(ref e)) if e.kind() == io::ErrorKind::Interrupted => {}
                Poll::Ready(Err(e)) => {
                    ret = Err(e);
                    break;
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        if written > 0 {
            buf.drain(..written);
        }
        Poll::Ready(ret)
    }

    pub fn poll_into_inner(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        //TODO: Fix 'expected function, found struct `IntoInnerError`' compiler error
    ) -> Poll<io::Result<W>> {
        match ready!(self.as_mut().poll_flush_buf(cx)) {
            Ok(()) => Poll::Ready(Ok(self.inner().take().unwrap())),
            Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, "")))
        }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for BufWriter<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let panicked = self.as_mut().panicked();
        if self.as_ref().buf.len() + buf.len() > self.as_ref().buf.capacity() {
            match ready!(self.as_mut().poll_flush_buf(cx)) {
                Ok(()) => {},
                Err(e) => return Poll::Ready(Err(e))
            }
        }
        if buf.len() >= self.as_ref().buf.capacity() {
            *panicked = true;
            let r = ready!(self.as_mut().poll_write(cx, buf));
            *panicked = false;
            return Poll::Ready(r)
        } else {
            return Poll::Ready(ready!(self.as_ref().buf.write(buf).poll()))
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        unimplemented!()
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        unimplemented!()
    }
}

impl<W: AsyncWrite + fmt::Debug> fmt::Debug for BufWriter<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufReader")
            .field("writer", &self.inner)
            .field(
                "buf",
                &self.buf
            )
            .finish()
    }
}

mod tests {

}