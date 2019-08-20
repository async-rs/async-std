use crate::task::{Context, Poll};
use futures::{ready, AsyncWrite, Future, Stream};
use std::io::{self, IntoInnerError};
use std::pin::Pin;
use std::fmt;
use crate::io::Write;

const DEFAULT_CAPACITY: usize = 8 * 1024;

/// Wraps a writer and buffers its output.
///
/// It can be excessively inefficient to work directly with something that
/// implements [`Write`]. For example, every call to
/// [`write`][`TcpStream::write`] on [`TcpStream`] results in a system call. A
/// `BufWriter` keeps an in-memory buffer of data and writes it to an underlying
/// writer in large, infrequent batches.
///
/// `BufWriter` can improve the speed of programs that make *small* and
/// *repeated* write calls to the same file or network socket. It does not
/// help when writing very large amounts at once, or writing just one or a few
/// times. It also provides no advantage when writing to a destination that is
/// in memory, like a `Vec<u8>`.
///
/// When the `BufWriter` is dropped, the contents of its buffer will be written
/// out. However, any errors that happen in the process of flushing the buffer
/// when the writer is dropped will be ignored. Code that wishes to handle such
/// errors must manually call [`flush`] before the writer is dropped.
///
/// This type is an async version of [`std::io::BufReader`].
///
/// [`std::io::BufReader`]: https://doc.rust-lang.org/std/io/struct.BufReader.html
///
/// # Examples
///
/// Let's write the numbers one through ten to a [`TcpStream`]:
///
/*/ ```no_run
/ use std::io::prelude::*;
/ use std::net::TcpStream;
/
/ let mut stream = TcpStream::connect("127.0.0.1:34254").unwrap();
/
/ for i in 0..10 {
/     stream.write(&[i+1]).unwrap();
/ }
/ ```*/
///
/// Because we're not buffering, we write each one in turn, incurring the
/// overhead of a system call per byte written. We can fix this with a
/// `BufWriter`:
///
/// ```no_run
/// use async_std::io::prelude::*;
/// use async_std::io::BufWriter;
/// use async_std::net::TcpStream;
/// use futures::AsyncWrite;
/// use async_std::io::Write;
///
/// async_std::task::block_on(async {
///     let mut stream = BufWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
///     for i in 0..10 {
///         stream.write(&[i+1]).await;
///     }
/// });
/// ```
///
/// By wrapping the stream with a `BufWriter`, these ten writes are all grouped
/// together by the buffer, and will all be written out in one system call when
/// the `stream` is dropped.
///
/// [`Write`]: ../../std/io/trait.Write.html
/// [`TcpStream::write`]: ../../std/net/struct.TcpStream.html#method.write
/// [`TcpStream`]: ../../std/net/struct.TcpStream.html
/// [`flush`]: #method.flush
pub struct BufWriter<W> {
    inner: W,
    buf: Vec<u8>,
    written: usize,
}

impl<W: AsyncWrite + Unpin> BufWriter<W> {
    pin_utils::unsafe_pinned!(inner: W);
    pin_utils::unsafe_unpinned!(buf: Vec<u8>);

    /// Creates a new `BufWriter` with a default buffer capacity. The default is currently 8 KB,
    /// but may change in the future.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use async_std::io::BufWriter;
    /// use async_std::net::TcpStream;
    ///
    /// let mut buffer = BufWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
    /// ```
    pub fn new(inner: W) -> BufWriter<W> {
        BufWriter::with_capacity(DEFAULT_CAPACITY, inner)
    }

    /// Creates a new `BufWriter` with the specified buffer capacity.
    ///
    /// # Examples
    ///
    /// Creating a buffer with a buffer of a hundred bytes.
    ///
    /// ```no_run
    /// use async_std::io::BufWriter;
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:34254").unwrap();
    /// let mut buffer = BufWriter::with_capacity(100, stream);
    /// ```
    pub fn with_capacity(capacity: usize, inner: W) -> BufWriter<W> {
        BufWriter {
            inner,
            buf: Vec::with_capacity(capacity),
            written: 0,
        }
    }

    /// Gets a reference to the underlying writer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use async_std::io::BufWriter;
    /// use async_std::net::TcpStream;
    ///
    /// let mut buffer = BufWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
    ///
    /// // we can use reference just like buffer
    /// let reference = buffer.get_ref();
    /// ```
    pub fn get_ref(&self) -> &W {
        &self.inner
    }

    /// Gets a mutable reference to the underlying writer.
    ///
    /// It is inadvisable to directly write to the underlying writer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use async_std::io::BufWriter;
    /// use async_std::net::TcpStream;
    ///
    /// let mut buffer = BufWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
    ///
    /// // we can use reference just like buffer
    /// let reference = buffer.get_mut();
    /// ```
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut W> {
        self.inner()
    }

    /// Consumes BufWriter, returning the underlying writer
    ///
    /// This method will not write leftover data, it will be lost.
    /// For method that will attempt to write before returning the writer see [`poll_into_inner`]
    ///
    /// [`poll_into_inner`]: #method.poll_into_inner
    pub fn into_inner(self) -> W {
        self.inner
    }

    pub fn poll_into_inner(mut self: Pin<&mut Self>, cx: Context<'_>) -> Poll<io::Result<usize>> {
        unimplemented!("poll into inner method")
    }

    /// Returns a reference to the internally buffered data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use async_std::io::BufWriter;
    /// use async_std::net::TcpStream;
    ///
    /// let buf_writer = BufWriter::new(TcpStream::connect("127.0.0.1:34254").unwrap());
    ///
    /// // See how many bytes are currently buffered
    /// let bytes_buffered = buf_writer.buffer().len();
    /// ```
    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }

    pub fn poll_flush_buf(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let Self {
            inner,
            buf,
            written
        } = Pin::get_mut(self);
        let mut inner = Pin::new(inner);
        let len = buf.len();
        let mut ret = Ok(());
        while *written < len {
            match inner.as_mut().poll_write(cx, &buf[*written..]) {
                Poll::Ready(Ok(0)) => {
                    ret = Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "Failed to write buffered data",
                    ));
                    break;
                }
                Poll::Ready(Ok(n)) => *written += n,
                Poll::Ready(Err(ref e)) if e.kind() == io::ErrorKind::Interrupted => {}
                Poll::Ready(Err(e)) => {
                    ret = Err(e);
                    break;
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        if *written > 0 {
            buf.drain(..*written);
        }
        *written = 0;
        Poll::Ready(ret)
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for BufWriter<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.buf.len() + buf.len() > self.buf.capacity() {
            ready!(self.as_mut().poll_flush_buf(cx))?;
        }
        if buf.len() >= self.buf.capacity() {
            self.inner().poll_write(cx, buf)
        } else {
            self.buf().write(buf).poll(cx)
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        ready!(self.as_mut().poll_flush_buf(cx))?;
        self.inner().poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        ready!(self.as_mut().poll_flush_buf(cx))?;
        self.inner().poll_close(cx)
    }
}

impl<W: AsyncWrite + fmt::Debug + Unpin> fmt::Debug for BufWriter<W> {
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

pub struct LineWriter<W: AsyncWrite + Unpin> {
    inner: BufWriter<W>,
    need_flush: bool,
}

impl<W: AsyncWrite + Unpin> LineWriter<W> {
    pin_utils::unsafe_pinned!(inner: BufWriter<W>);
    pin_utils::unsafe_unpinned!(need_flush: bool);
    /// Creates a new `LineWriter`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use async_std::fs::File;
    /// use async_std::io::LineWriter;
    ///
    /// fn main() -> std::io::Result<()> {
    /// async_std::task::block_on(async {
    ///         let file = File::create("poem.txt").await?;
    ///         let file = LineWriter::new(file);
    ///         Ok(())
    ///     })
    /// }
    /// ```
    pub fn new(inner: W) -> LineWriter<W> {
        // Lines typically aren't that long, don't use a giant buffer
        LineWriter::with_capacity(1024, inner)
    }

    /// Creates a new `LineWriter` with a specified capacity for the internal
    /// buffer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use async_std::fs::File;
    /// use async_std::io::LineWriter;
    ///
    /// fn main() -> std::io::Result<()> {
    ///     async_std::task::block_on(async {
    ///         let file = File::create("poem.txt").await?;
    ///         let file = LineWriter::with_capacity(100, file);
    ///         Ok(())
    ///     })
    /// }
    /// ```
    pub fn with_capacity(capacity: usize, inner: W) -> LineWriter<W> {
        LineWriter {
            inner: BufWriter::with_capacity(capacity, inner),
            need_flush: false,
        }
    }

    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }

    pub fn get_mut(&mut self) -> &mut W {
        self.inner.get_mut()
    }

    pub fn into_inner(self) -> W {
        self.inner.into_inner()
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for LineWriter<W> {
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        if self.need_flush {
            self.as_mut().poll_flush(cx)?;
        }

        let i = match memchr::memrchr(b'\n', buf) {
            Some(i) => i,
            None => return self.as_mut().inner().as_mut().poll_write(cx, buf)
        };

        let n = ready!(self.as_mut().inner().as_mut().poll_write(cx, &buf[..=i])?);
        *self.as_mut().need_flush() = true;
        if ready!(self.as_mut().poll_flush(cx)).is_err() || n != 1 + 1 {
            return Poll::Ready(Ok(n))
        }
        match ready!(self.inner().poll_write(cx, &buf[i + 1..])) {
            Ok(i) => Poll::Ready(Ok(n + 1)),
            Err(_) => Poll::Ready(Ok(n))
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        self.as_mut().inner().poll_flush(cx)?;
        *self.need_flush() = false;
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        self.as_mut().inner().poll_flush(cx)?;
        self.inner().poll_close(cx)
    }
}

impl<W: AsyncWrite + Unpin> fmt::Debug for LineWriter<W> where W: fmt::Debug {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("LineWriter")
            .field("writer", &self.inner.inner)
            .field("buffer",
                   &format_args!("{}/{}", self.inner.buf.len(), self.inner.buf.capacity()))
            .finish()
    }
}


mod tests {
    use crate::prelude::*;
    use crate::task;
    use super::LineWriter;

    #[test]
    fn test_line_buffer() {
        task::block_on(async {
            let mut writer = LineWriter::new(Vec::new());
            writer.write(&[0]).await.unwrap();
            assert_eq!(*writer.get_ref(), []);
            writer.write(&[1]).await.unwrap();
            assert_eq!(*writer.get_ref(), []);
            writer.flush().await.unwrap();
            assert_eq!(*writer.get_ref(), [0, 1]);
            writer.write(&[0, b'\n', 1, b'\n', 2]).await.unwrap();
            assert_eq!(*writer.get_ref(), [0, 1, 0, b'\n', 1, b'\n']);
            writer.flush().await.unwrap();
            assert_eq!(*writer.get_ref(), [0, 1, 0, b'\n', 1, b'\n', 2]);
            writer.write(&[3, b'\n']).await.unwrap();
            assert_eq!(*writer.get_ref(), [0, 1, 0, b'\n', 1, b'\n', 2, 3, b'\n']);
        })
    }
}