use crate::task::{Context, Poll};
use futures::{ready, AsyncWrite};
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
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// use async_std::net::TcpStream;
/// use async_std::io::Write;
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
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// use async_std::io::BufWriter;
/// use async_std::net::TcpStream;
/// use async_std::io::Write;
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

impl<W: AsyncWrite + Unpin> BufWriter<W> {
    pin_utils::unsafe_pinned!(inner: W);
    pin_utils::unsafe_unpinned!(buf: Vec<u8>);

    /// Creates a new `BufWriter` with a default buffer capacity. The default is currently 8 KB,
    /// but may change in the future.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
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
    /// # #![feature(async_await)]
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
    /// # #![feature(async_await)]
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
    /// # #![feature(async_await)]
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
    /// # #![feature(async_await)]
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
    pub fn poll_flush_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let Self {
            inner,
            buf,
            written,
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

impl<W: AsyncWrite + fmt::Debug + Unpin> fmt::Debug for BufWriter<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufReader")
            .field("writer", &self.inner)
            .field("buf", &self.buf)
            .finish()
    }
}

/// Wraps a writer and buffers output to it, flushing whenever a newline
/// (`0x0a`, `'\n'`) is detected.
///
/// The [`BufWriter`][bufwriter] struct wraps a writer and buffers its output.
/// But it only does this batched write when it goes out of scope, or when the
/// internal buffer is full. Sometimes, you'd prefer to write each line as it's
/// completed, rather than the entire buffer at once. Enter `LineWriter`. It
/// does exactly that.
///
/// Like [`BufWriter`][bufwriter], a `LineWriter`’s buffer will also be flushed when the
/// `LineWriter` goes out of scope or when its internal buffer is full.
///
/// [bufwriter]: struct.BufWriter.html
///
/// If there's still a partial line in the buffer when the `LineWriter` is
/// dropped, it will flush those contents.
///
/// This type is an async version of [`std::io::LineWriter`]
///
/// [`std::io::LineWriter`]: https://doc.rust-lang.org/std/io/struct.LineWriter.html
///
/// # Examples
///
/// We can use `LineWriter` to write one line at a time, significantly
/// reducing the number of actual writes to the file.
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
///     use async_std::io::{LineWriter, Write};
///     use async_std::fs::{File, self};
///     let road_not_taken = b"I shall be telling this with a sigh
/// Somewhere ages and ages hence:
/// Two roads diverged in a wood, and I -
/// I took the one less traveled by,
/// And that has made all the difference.";
///
///     let file = File::create("poem.txt").await?;
///     let mut file = LineWriter::new(file);
///
///     file.write_all(b"I shall be telling this with a sigh").await?;
///
///     // No bytes are written until a newline is encountered (or
///     // the internal buffer is filled).
///     assert_eq!(fs::read_to_string("poem.txt").await?, "");
///     file.write_all(b"\n").await?;
///     assert_eq!(
///         fs::read_to_string("poem.txt").await?,
///         "I shall be telling this with a sigh\n",
///     );
///
///     // Write the rest of the poem.
///     file.write_all(b"Somewhere ages and ages hence:
/// Two roads diverged in a wood, and I -
/// I took the one less traveled by,
/// And that has made all the difference.").await?;
///
///     // The last line of the poem doesn't end in a newline, so
///     // we have to flush or drop the `LineWriter` to finish
///     // writing.
///     file.flush().await?;
///
///     // Confirm the whole poem was written.
///     assert_eq!(fs::read("poem.txt").await?, &road_not_taken[..]);
/// #
/// # Ok(()) }) }
/// ```
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
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// use async_std::fs::File;
    /// use async_std::io::LineWriter;
    ///
    /// let file = File::create("poem.txt").await?;
    /// let file = LineWriter::new(file);
    /// #
    /// # Ok(()) }) }
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
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// use async_std::fs::File;
    /// use async_std::io::LineWriter;
    ///
    /// let file = File::create("poem.txt").await?;
    /// let file = LineWriter::with_capacity(100, file);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn with_capacity(capacity: usize, inner: W) -> LineWriter<W> {
        LineWriter {
            inner: BufWriter::with_capacity(capacity, inner),
            need_flush: false,
        }
    }

    /// Gets a reference to the underlying writer.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # #![allow(unused_mut)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// use async_std::io::LineWriter;
    /// use async_std::fs::File;
    ///
    /// let file = File::create("poem.txt").await?;
    /// let file = LineWriter::new(file);
    ///
    /// // We can use reference just like buffer
    /// let reference = file.get_ref();
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn get_ref(&self) -> &W {
        self.inner.get_ref()
    }

    /// Gets a mutable reference to the underlying writer.
    ///
    /// Caution must be taken when calling methods on the mutable reference
    /// returned as extra writes could corrupt the output stream.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # #![allow(unused_mut)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// use async_std::io::LineWriter;
    /// use async_std::fs::File;
    ///
    /// let file = File::create("poem.txt").await?;
    /// let mut file = LineWriter::new(file);
    ///
    /// // We can use reference just like buffer
    /// let reference = file.get_mut();
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn get_mut(&mut self) -> &mut W {
        self.inner.get_mut()
    }

    //TODO: Implement flushing before returning inner
    #[allow(missing_docs)]
    pub fn into_inner(self) -> W {
        self.inner.into_inner()
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for LineWriter<W> {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.need_flush {
            let _ = self.as_mut().poll_flush(cx)?;
        }

        let i = match memchr::memrchr(b'\n', buf) {
            Some(i) => i,
            None => return self.as_mut().inner().as_mut().poll_write(cx, buf),
        };

        let n = ready!(self.as_mut().inner().as_mut().poll_write(cx, &buf[..=i])?);
        *self.as_mut().need_flush() = true;
        if ready!(self.as_mut().poll_flush(cx)).is_err() || n != i + 1 {
            return Poll::Ready(Ok(n));
        }
        match ready!(self.inner().poll_write(cx, &buf[i + 1..])) {
            Ok(_) => Poll::Ready(Ok(n + 1)),
            Err(_) => Poll::Ready(Ok(n)),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let _ = self.as_mut().inner().poll_flush(cx)?;
        *self.need_flush() = false;
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let _ = self.as_mut().inner().poll_flush(cx)?;
        self.inner().poll_close(cx)
    }
}

impl<W: AsyncWrite + Unpin> fmt::Debug for LineWriter<W>
where
    W: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("LineWriter")
            .field("writer", &self.inner.inner)
            .field(
                "buffer",
                &format_args!("{}/{}", self.inner.buf.len(), self.inner.buf.capacity()),
            )
            .finish()
    }
}

mod tests {
    #![allow(unused_imports)]
    use super::LineWriter;
    use crate::prelude::*;
    use crate::task;

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
