use std::io::IoSliceMut;
use std::mem;
use std::pin::Pin;
use std::str;

use cfg_if::cfg_if;
use futures::io::AsyncRead;

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

/// Allows reading from a byte stream.
///
/// This trait is an async version of [`std::io::Read`].
///
/// While it is currently not possible to implement this trait directly, it gets implemented
/// automatically for all types that implement [`futures::io::AsyncRead`].
///
/// [`std::io::Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
/// [`futures::io::AsyncRead`]:
/// https://docs/futures-preview/0.3.0-alpha.17/futures/io/trait.AsyncRead.html
pub trait Read {
    /// Reads some bytes from the byte stream.
    ///
    /// Returns the number of bytes read from the start of the buffer.
    ///
    /// If the return value is `Ok(n)`, then it must be guaranteed that `0 <= n <= buf.len()`. A
    /// nonzero `n` value indicates that the buffer has been filled in with `n` bytes of data. If
    /// `n` is `0`, then it can indicate one of two scenarios:
    ///
    /// 1. This reader has reached its "end of file" and will likely no longer be able to produce
    ///    bytes. Note that this does not mean that the reader will always no longer be able to
    ///    produce bytes.
    /// 2. The buffer specified was 0 bytes in length.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::prelude::*;
    ///
    /// let mut f = File::open("a.txt").await?;
    ///
    /// let mut buf = vec![0; 1024];
    /// let n = f.read(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ret!('a, ReadFuture, io::Result<usize>)
    where
        Self: Unpin;

    /// Like [`read`], except that it reads into a slice of buffers.
    ///
    /// Data is copied to fill each buffer in order, with the final buffer written to possibly
    /// being only partially filled. This method must behave as a single call to [`read`] with the
    /// buffers concatenated would.
    ///
    /// The default implementation calls [`read`] with either the first nonempty buffer provided,
    /// or an empty one if none exists.
    ///
    /// [`read`]: #tymethod.read
    fn read_vectored<'a>(
        &'a mut self,
        bufs: &'a mut [IoSliceMut<'a>],
    ) -> ret!('a, ReadVectoredFuture, io::Result<usize>)
    where
        Self: Unpin,
    {
        ReadVectoredFuture { reader: self, bufs }
    }

    /// Reads all bytes from the byte stream.
    ///
    /// All bytes read from this stream will be appended to the specified buffer `buf`. This
    /// function will continuously call [`read`] to append more data to `buf` until [`read`]
    /// returns either `Ok(0)` or an error.
    ///
    /// If successful, this function will return the total number of bytes read.
    ///
    /// [`read`]: #tymethod.read
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::prelude::*;
    ///
    /// let mut f = File::open("a.txt").await?;
    ///
    /// let mut buf = Vec::new();
    /// f.read_to_end(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> ret!('a, ReadToEndFuture, io::Result<usize>)
    where
        Self: Unpin,
    {
        let start_len = buf.len();
        ReadToEndFuture {
            reader: self,
            buf,
            start_len,
        }
    }

    /// Reads all bytes from the byte stream and appends them into a string.
    ///
    /// If successful, this function will return the number of bytes read.
    ///
    /// If the data in this stream is not valid UTF-8 then an error will be returned and `buf` will
    /// be left unmodified.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::prelude::*;
    ///
    /// let mut f = File::open("a.txt").await?;
    ///
    /// let mut buf = String::new();
    /// f.read_to_string(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    fn read_to_string<'a>(
        &'a mut self,
        buf: &'a mut String,
    ) -> ret!('a, ReadToStringFuture, io::Result<usize>)
    where
        Self: Unpin,
    {
        let start_len = buf.len();
        ReadToStringFuture {
            reader: self,
            bytes: unsafe { mem::replace(buf.as_mut_vec(), Vec::new()) },
            buf,
            start_len,
        }
    }

    /// Reads the exact number of bytes required to fill `buf`.
    ///
    /// This function reads as many bytes as necessary to completely fill the specified buffer
    /// `buf`.
    ///
    /// No guarantees are provided about the contents of `buf` when this function is called,
    /// implementations cannot rely on any property of the contents of `buf` being true. It is
    /// recommended that implementations only write data to `buf` instead of reading its contents.
    ///
    /// If this function encounters an "end of file" before completely filling the buffer, it
    /// returns an error of the kind [`ErrorKind::UnexpectedEof`].  The contents of `buf` are
    /// unspecified in this case.
    ///
    /// If any other read error is encountered then this function immediately returns. The contents
    /// of `buf` are unspecified in this case.
    ///
    /// If this function returns an error, it is unspecified how many bytes it has read, but it
    /// will never read more than would be necessary to completely fill the buffer.
    ///
    /// [`ErrorKind::UnexpectedEof`]: enum.ErrorKind.html#variant.UnexpectedEof
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::prelude::*;
    ///
    /// let mut f = File::open("a.txt").await?;
    ///
    /// let mut buf = vec![0; 10];
    /// f.read_exact(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> ret!('a, ReadExactFuture, io::Result<()>)
    where
        Self: Unpin,
    {
        ReadExactFuture { reader: self, buf }
    }
}

impl<T: AsyncRead + Unpin + ?Sized> Read for T {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ret!('a, ReadFuture, io::Result<usize>) {
        ReadFuture { reader: self, buf }
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ReadFuture<'a, T: Unpin + ?Sized> {
    reader: &'a mut T,
    buf: &'a mut [u8],
}

impl<T: AsyncRead + Unpin + ?Sized> Future for ReadFuture<'_, T> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self { reader, buf } = &mut *self;
        Pin::new(reader).poll_read(cx, buf)
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ReadVectoredFuture<'a, T: Unpin + ?Sized> {
    reader: &'a mut T,
    bufs: &'a mut [IoSliceMut<'a>],
}

impl<T: AsyncRead + Unpin + ?Sized> Future for ReadVectoredFuture<'_, T> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self { reader, bufs } = &mut *self;
        Pin::new(reader).poll_read_vectored(cx, bufs)
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ReadToEndFuture<'a, T: Unpin + ?Sized> {
    reader: &'a mut T,
    buf: &'a mut Vec<u8>,
    start_len: usize,
}

impl<T: AsyncRead + Unpin + ?Sized> Future for ReadToEndFuture<'_, T> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            reader,
            buf,
            start_len,
        } = &mut *self;
        read_to_end_internal(Pin::new(reader), cx, buf, *start_len)
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ReadToStringFuture<'a, T: Unpin + ?Sized> {
    reader: &'a mut T,
    buf: &'a mut String,
    bytes: Vec<u8>,
    start_len: usize,
}

impl<T: AsyncRead + Unpin + ?Sized> Future for ReadToStringFuture<'_, T> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {
            reader,
            buf,
            bytes,
            start_len,
        } = &mut *self;
        let reader = Pin::new(reader);

        let ret = futures::ready!(read_to_end_internal(reader, cx, bytes, *start_len));
        if str::from_utf8(&bytes).is_err() {
            Poll::Ready(ret.and_then(|_| {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "stream did not contain valid UTF-8",
                ))
            }))
        } else {
            debug_assert!(buf.is_empty());
            // Safety: `bytes` is a valid UTF-8 because `str::from_utf8` returned `Ok`.
            mem::swap(unsafe { buf.as_mut_vec() }, bytes);
            Poll::Ready(ret)
        }
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct ReadExactFuture<'a, T: Unpin + ?Sized> {
    reader: &'a mut T,
    buf: &'a mut [u8],
}

impl<T: AsyncRead + Unpin + ?Sized> Future for ReadExactFuture<'_, T> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self { reader, buf } = &mut *self;

        while !buf.is_empty() {
            let n = futures::ready!(Pin::new(&mut *reader).poll_read(cx, buf))?;
            let (_, rest) = mem::replace(buf, &mut []).split_at_mut(n);
            *buf = rest;

            if n == 0 {
                return Poll::Ready(Err(io::ErrorKind::UnexpectedEof.into()));
            }
        }

        Poll::Ready(Ok(()))
    }
}

// This uses an adaptive system to extend the vector when it fills. We want to
// avoid paying to allocate and zero a huge chunk of memory if the reader only
// has 4 bytes while still making large reads if the reader does have a ton
// of data to return. Simply tacking on an extra DEFAULT_BUF_SIZE space every
// time is 4,500 times (!) slower than this if the reader has a very small
// amount of data to return.
//
// Because we're extending the buffer with uninitialized data for trusted
// readers, we need to make sure to truncate that if any of this panics.
pub fn read_to_end_internal<R: AsyncRead + ?Sized>(
    mut rd: Pin<&mut R>,
    cx: &mut Context<'_>,
    buf: &mut Vec<u8>,
    start_len: usize,
) -> Poll<io::Result<usize>> {
    struct Guard<'a> {
        buf: &'a mut Vec<u8>,
        len: usize,
    }

    impl Drop for Guard<'_> {
        fn drop(&mut self) {
            unsafe {
                self.buf.set_len(self.len);
            }
        }
    }

    let mut g = Guard {
        len: buf.len(),
        buf,
    };
    let ret;
    loop {
        if g.len == g.buf.len() {
            unsafe {
                g.buf.reserve(32);
                let capacity = g.buf.capacity();
                g.buf.set_len(capacity);
                rd.initializer().initialize(&mut g.buf[g.len..]);
            }
        }

        match futures::ready!(rd.as_mut().poll_read(cx, &mut g.buf[g.len..])) {
            Ok(0) => {
                ret = Poll::Ready(Ok(g.len - start_len));
                break;
            }
            Ok(n) => g.len += n,
            Err(e) => {
                ret = Poll::Ready(Err(e));
                break;
            }
        }
    }

    ret
}
