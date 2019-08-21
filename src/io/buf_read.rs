use std::mem;
use std::pin::Pin;
use std::str;

use cfg_if::cfg_if;
use futures::io::AsyncBufRead;

use crate::future::Future;
use crate::io;
use crate::task::{Context, Poll};

cfg_if! {
    if #[cfg(feature = "docs")] {
        #[doc(hidden)]
        pub struct ImplFuture<'a, T>(std::marker::PhantomData<&'a T>);

        macro_rules! ret {
            ($a:lifetime, $f:tt, $o:ty) => (ImplFuture<$a, $o>);
        }
    } else {
        macro_rules! ret {
            ($a:lifetime, $f:tt, $o:ty) => ($f<$a, Self>);
        }
    }
}

/// Allows reading from a buffered byte stream.
///
/// This trait is an async version of [`std::io::BufRead`].
///
/// While it is currently not possible to implement this trait directly, it gets implemented
/// automatically for all types that implement [`futures::io::AsyncBufRead`].
///
/// [`std::io::BufRead`]: https://doc.rust-lang.org/std/io/trait.BufRead.html
/// [`futures::io::AsyncBufRead`]:
/// https://docs.rs/futures-preview/0.3.0-alpha.17/futures/io/trait.AsyncBufRead.html
pub trait BufRead {
    /// Reads all bytes into `buf` until the delimiter `byte` or EOF is reached.
    ///
    /// This function will read bytes from the underlying stream until the delimiter or EOF is
    /// found. Once found, all bytes up to, and including, the delimiter (if found) will be
    /// appended to `buf`.
    ///
    /// If successful, this function will return the total number of bytes read.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::io::BufReader;
    /// use async_std::prelude::*;
    ///
    /// let mut file = BufReader::new(File::open("a.txt").await?);
    ///
    /// let mut buf = vec![0; 1024];
    /// let n = file.read_until(b'\n', &mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    fn read_until<'a>(
        &'a mut self,
        byte: u8,
        buf: &'a mut Vec<u8>,
    ) -> ret!('a, ReadUntilFuture, io::Result<usize>)
    where
        Self: Unpin,
    {
        ReadUntilFuture {
            reader: self,
            byte,
            buf,
            read: 0,
        }
    }

    /// Reads all bytes and appends them into `buf` until a newline (the 0xA byte) is reached.
    ///
    /// This function will read bytes from the underlying stream until the newline delimiter (the
    /// 0xA byte) or EOF is found. Once found, all bytes up to, and including, the delimiter (if
    /// found) will be appended to `buf`.
    ///
    /// If successful, this function will return the total number of bytes read.
    ///
    /// If this function returns `Ok(0)`, the stream has reached EOF.
    ///
    /// # Errors
    ///
    /// This function has the same error semantics as [`read_until`] and will also return an error
    /// if the read bytes are not valid UTF-8. If an I/O error is encountered then `buf` may
    /// contain some bytes already read in the event that all data read so far was valid UTF-8.
    ///
    /// [`read_until`]: #method.read_until
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::io::BufReader;
    /// use async_std::prelude::*;
    ///
    /// let mut file = BufReader::new(File::open("a.txt").await?);
    ///
    /// let mut buf = String::new();
    /// file.read_line(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    fn read_line<'a>(
        &'a mut self,
        buf: &'a mut String,
    ) -> ret!('a, ReadLineFuture, io::Result<usize>)
    where
        Self: Unpin,
    {
        ReadLineFuture {
            reader: self,
            bytes: unsafe { mem::replace(buf.as_mut_vec(), Vec::new()) },
            buf,
            read: 0,
        }
    }

    /// Returns a stream over the lines of this byte stream.
    ///
    /// The stream returned from this function will yield instances of
    /// [`io::Result`]`<`[`String`]`>`. Each string returned will *not* have a newline byte (the
    /// 0xA byte) or CRLF (0xD, 0xA bytes) at the end.
    ///
    /// [`io::Result`]: type.Result.html
    /// [`String`]: https://doc.rust-lang.org/std/string/struct.String.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::io::BufReader;
    /// use async_std::prelude::*;
    ///
    /// let file = File::open("a.txt").await?;
    /// let mut lines = BufReader::new(file).lines();
    /// let mut count = 0;
    ///
    /// for line in lines.next().await {
    ///     line?;
    ///     count += 1;
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    fn lines(self) -> Lines<Self>
    where
        Self: Unpin + Sized,
    {
        Lines {
            reader: self,
            buf: String::new(),
            bytes: Vec::new(),
            read: 0,
        }
    }
}

impl<T: AsyncBufRead + Unpin + ?Sized> BufRead for T {}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ReadUntilFuture<'a, T: Unpin + ?Sized> {
    reader: &'a mut T,
    byte: u8,
    buf: &'a mut Vec<u8>,
    read: usize,
}

impl<T: AsyncBufRead + Unpin + ?Sized> Future for ReadUntilFuture<'_, T> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            reader,
            byte,
            buf,
            read,
        } = &mut *self;
        read_until_internal(Pin::new(reader), cx, *byte, buf, read)
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ReadLineFuture<'a, T: Unpin + ?Sized> {
    reader: &'a mut T,
    buf: &'a mut String,
    bytes: Vec<u8>,
    read: usize,
}

impl<T: AsyncBufRead + Unpin + ?Sized> Future for ReadLineFuture<'_, T> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            reader,
            buf,
            bytes,
            read,
        } = &mut *self;
        let reader = Pin::new(reader);

        let ret = futures::ready!(read_until_internal(reader, cx, b'\n', bytes, read));
        if str::from_utf8(&bytes).is_err() {
            Poll::Ready(ret.and_then(|_| {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "stream did not contain valid UTF-8",
                ))
            }))
        } else {
            debug_assert!(buf.is_empty());
            debug_assert_eq!(*read, 0);
            // Safety: `bytes` is a valid UTF-8 because `str::from_utf8` returned `Ok`.
            mem::swap(unsafe { buf.as_mut_vec() }, bytes);
            Poll::Ready(ret)
        }
    }
}

/// A stream of lines in a byte stream.
///
/// This stream is created by the [`lines`] method on types that implement [`BufRead`].
///
/// This type is an async version of [`std::io::Lines`].
///
/// [`lines`]: trait.BufRead.html#method.lines
/// [`BufRead`]: trait.BufRead.html
/// [`std::io::Lines`]: https://doc.rust-lang.org/nightly/std/io/struct.Lines.html
#[derive(Debug)]
pub struct Lines<R> {
    reader: R,
    buf: String,
    bytes: Vec<u8>,
    read: usize,
}

impl<R: AsyncBufRead> futures::Stream for Lines<R> {
    type Item = io::Result<String>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Self {
            reader,
            buf,
            bytes,
            read,
        } = unsafe { self.get_unchecked_mut() };
        let reader = unsafe { Pin::new_unchecked(reader) };
        let n = futures::ready!(read_line_internal(reader, cx, buf, bytes, read))?;
        if n == 0 && buf.is_empty() {
            return Poll::Ready(None);
        }
        if buf.ends_with('\n') {
            buf.pop();
            if buf.ends_with('\r') {
                buf.pop();
            }
        }
        Poll::Ready(Some(Ok(mem::replace(buf, String::new()))))
    }
}

pub fn read_line_internal<R: AsyncBufRead + ?Sized>(
    reader: Pin<&mut R>,
    cx: &mut Context<'_>,
    buf: &mut String,
    bytes: &mut Vec<u8>,
    read: &mut usize,
) -> Poll<io::Result<usize>> {
    let ret = futures::ready!(read_until_internal(reader, cx, b'\n', bytes, read));
    if str::from_utf8(&bytes).is_err() {
        Poll::Ready(ret.and_then(|_| {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            ))
        }))
    } else {
        debug_assert!(buf.is_empty());
        debug_assert_eq!(*read, 0);
        // Safety: `bytes` is a valid UTF-8 because `str::from_utf8` returned `Ok`.
        mem::swap(unsafe { buf.as_mut_vec() }, bytes);
        Poll::Ready(ret)
    }
}

pub fn read_until_internal<R: AsyncBufRead + ?Sized>(
    mut reader: Pin<&mut R>,
    cx: &mut Context<'_>,
    byte: u8,
    buf: &mut Vec<u8>,
    read: &mut usize,
) -> Poll<io::Result<usize>> {
    loop {
        let (done, used) = {
            let available = futures::ready!(reader.as_mut().poll_fill_buf(cx))?;
            if let Some(i) = memchr::memchr(byte, available) {
                buf.extend_from_slice(&available[..=i]);
                (true, i + 1)
            } else {
                buf.extend_from_slice(available);
                (false, available.len())
            }
        };
        reader.as_mut().consume(used);
        *read += used;
        if done || used == 0 {
            return Poll::Ready(Ok(mem::replace(read, 0)));
        }
    }
}
