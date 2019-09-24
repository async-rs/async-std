use crate::task::{Context, Poll};
use futures_core::ready;
use futures_io::{AsyncSeek, AsyncWrite, SeekFrom};
use std::fmt;
use std::io;
use std::pin::Pin;

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
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// use async_std::net::TcpStream;
/// use async_std::prelude::*;
///
/// let mut stream = TcpStream::connect("127.0.0.1:34254").await?;
///
/// for i in 0..10 {
///     let arr = [i+1];
///     stream.write(&arr).await?;
/// }
/// #
/// # Ok(()) }) }
/// ```
///
/// Because we're not buffering, we write each one in turn, incurring the
/// overhead of a system call per byte written. We can fix this with a
/// `BufWriter`:
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// use async_std::io::BufWriter;
/// use async_std::net::TcpStream;
/// use async_std::prelude::*;
///
/// let mut stream = BufWriter::new(TcpStream::connect("127.0.0.1:34254").await?);
/// for i in 0..10 {
///     let arr = [i+1];
///     stream.write(&arr).await?;
/// };
/// #
/// # Ok(()) }) }
/// ```
///
/// By wrapping the stream with a `BufWriter`, these ten writes are all grouped
/// together by the buffer, and will all be written out in one system call when
/// the `stream` is dropped.
///
/// [`Write`]: trait.Write.html
/// [`TcpStream::write`]: ../net/struct.TcpStream.html#method.write
/// [`TcpStream`]: ../net/struct.TcpStream.html
/// [`flush`]: trait.Write.html#tymethod.flush
pub struct BufWriter<W> {
    inner: W,
    buf: Vec<u8>,
    written: usize,
}

impl<W: AsyncWrite> BufWriter<W> {
    pin_utils::unsafe_pinned!(inner: W);
    pin_utils::unsafe_unpinned!(buf: Vec<u8>);

    /// Creates a new `BufWriter` with a default buffer capacity. The default is currently 8 KB,
    /// but may change in the future.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![allow(unused_mut)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// use async_std::io::BufWriter;
    /// use async_std::net::TcpStream;
    ///
    /// let mut buffer = BufWriter::new(TcpStream::connect("127.0.0.1:34254").await?);
    /// #
    /// # Ok(()) }) }
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
    /// # #![allow(unused_mut)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// use async_std::io::BufWriter;
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:34254").await?;
    /// let mut buffer = BufWriter::with_capacity(100, stream);
    /// #
    /// # Ok(()) }) }
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
    /// # #![allow(unused_mut)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// use async_std::io::BufWriter;
    /// use async_std::net::TcpStream;
    ///
    /// let mut buffer = BufWriter::new(TcpStream::connect("127.0.0.1:34254").await?);
    ///
    /// // We can use reference just like buffer
    /// let reference = buffer.get_ref();
    /// #
    /// # Ok(()) }) }
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
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// use async_std::io::BufWriter;
    /// use async_std::net::TcpStream;
    ///
    /// let mut buffer = BufWriter::new(TcpStream::connect("127.0.0.1:34254").await?);
    ///
    /// // We can use reference just like buffer
    /// let reference = buffer.get_mut();
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.inner
    }

    //    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut W> {
    //        self.inner()
    //    }

    /// Consumes BufWriter, returning the underlying writer
    ///
    /// This method will not write leftover data, it will be lost.
    /// For method that will attempt to write before returning the writer see [`poll_into_inner`]
    ///
    /// [`poll_into_inner`]: #method.poll_into_inner
    pub fn into_inner(self) -> W {
        self.inner
    }

    //    pub fn poll_into_inner(self: Pin<&mut Self>, _cx: Context<'_>) -> Poll<io::Result<usize>> {
    //        unimplemented!("poll into inner method")
    //    }

    /// Returns a reference to the internally buffered data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// use async_std::io::BufWriter;
    /// use async_std::net::TcpStream;
    ///
    /// let buf_writer = BufWriter::new(TcpStream::connect("127.0.0.1:34251").await?);
    ///
    /// // See how many bytes are currently buffered
    /// let bytes_buffered = buf_writer.buffer().len();
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn buffer(&self) -> &[u8] {
        &self.buf
    }

    /// Poll buffer flushing until completion
    ///
    /// This is used in types that wrap around BufWrite, one such example: [`LineWriter`]
    ///
    /// [`LineWriter`]: struct.LineWriter.html
    fn poll_flush_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let Self {
            inner,
            buf,
            written,
        } = unsafe { Pin::get_unchecked_mut(self) };
        let mut inner = unsafe { Pin::new_unchecked(inner) };
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

impl<W: AsyncWrite> AsyncWrite for BufWriter<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.buf.len() + buf.len() > self.buf.capacity() {
            ready!(self.as_mut().poll_flush_buf(cx))?;
        }
        if buf.len() >= self.buf.capacity() {
            self.inner().poll_write(cx, buf)
        } else {
            Pin::new(&mut *self.buf()).poll_write(cx, buf)
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        ready!(self.as_mut().poll_flush_buf(cx))?;
        self.inner().poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        ready!(self.as_mut().poll_flush_buf(cx))?;
        self.inner().poll_close(cx)
    }
}

impl<W: AsyncWrite + fmt::Debug> fmt::Debug for BufWriter<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufReader")
            .field("writer", &self.inner)
            .field("buf", &self.buf)
            .finish()
    }
}

impl<W: AsyncWrite + AsyncSeek> AsyncSeek for BufWriter<W> {
    /// Seek to the offset, in bytes, in the underlying writer.
    ///
    /// Seeking always writes out the internal buffer before seeking.

    fn poll_seek(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<io::Result<u64>> {
        ready!(self.as_mut().poll_flush_buf(cx))?;
        self.inner().poll_seek(cx, pos)
    }
}

mod tests {
    #![allow(unused_imports)]

    use super::BufWriter;
    use crate::io::{self, SeekFrom};
    use crate::prelude::*;
    use crate::task;

    #[test]
    fn test_buffered_writer() {
        task::block_on(async {
            let inner = Vec::new();
            let mut writer = BufWriter::with_capacity(2, inner);

            writer.write(&[0, 1]).await.unwrap();
            assert_eq!(writer.buffer(), []);
            assert_eq!(*writer.get_ref(), [0, 1]);

            writer.write(&[2]).await.unwrap();
            assert_eq!(writer.buffer(), [2]);
            assert_eq!(*writer.get_ref(), [0, 1]);

            writer.write(&[3]).await.unwrap();
            assert_eq!(writer.buffer(), [2, 3]);
            assert_eq!(*writer.get_ref(), [0, 1]);

            writer.flush().await.unwrap();
            assert_eq!(writer.buffer(), []);
            assert_eq!(*writer.get_ref(), [0, 1, 2, 3]);

            writer.write(&[4]).await.unwrap();
            writer.write(&[5]).await.unwrap();
            assert_eq!(writer.buffer(), [4, 5]);
            assert_eq!(*writer.get_ref(), [0, 1, 2, 3]);

            writer.write(&[6]).await.unwrap();
            assert_eq!(writer.buffer(), [6]);
            assert_eq!(*writer.get_ref(), [0, 1, 2, 3, 4, 5]);

            writer.write(&[7, 8]).await.unwrap();
            assert_eq!(writer.buffer(), []);
            assert_eq!(*writer.get_ref(), [0, 1, 2, 3, 4, 5, 6, 7, 8]);

            writer.write(&[9, 10, 11]).await.unwrap();
            assert_eq!(writer.buffer(), []);
            assert_eq!(*writer.get_ref(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);

            writer.flush().await.unwrap();
            assert_eq!(writer.buffer(), []);
            assert_eq!(*writer.get_ref(), [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
        })
    }

    #[test]
    fn test_buffered_writer_inner_into_inner_does_not_flush() {
        task::block_on(async {
            let mut w = BufWriter::with_capacity(3, Vec::new());
            w.write(&[0, 1]).await.unwrap();
            assert_eq!(*w.get_ref(), []);
            let w = w.into_inner();
            assert_eq!(w, []);
        })
    }

    #[test]
    fn test_buffered_writer_seek() {
        task::block_on(async {
            let mut w = BufWriter::with_capacity(3, io::Cursor::new(Vec::new()));
            w.write_all(&[0, 1, 2, 3, 4, 5]).await.unwrap();
            w.write_all(&[6, 7]).await.unwrap();
            assert_eq!(w.seek(SeekFrom::Current(0)).await.ok(), Some(8));
            assert_eq!(&w.get_ref().get_ref()[..], &[0, 1, 2, 3, 4, 5, 6, 7][..]);
            assert_eq!(w.seek(SeekFrom::Start(2)).await.ok(), Some(2));
        })
    }
}
