use std::io::{self, IoSlice};
use std::mem;
use std::pin::Pin;

use cfg_if::cfg_if;
use futures::io::AsyncWrite;

use crate::future::Future;
use crate::task::{Context, Poll};

cfg_if! {
    if #[cfg(feature = "docs.rs")] {
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

/// Allows writing to a byte stream.
///
/// This trait is an async version of [`std::io::Write`].
///
/// While it is currently not possible to implement this trait directly, it gets implemented
/// automatically for all types that implement [`futures::io::AsyncWrite`].
///
/// [`std::io::Write`]: https://doc.rust-lang.org/std/io/trait.Write.html
/// [`futures::io::AsyncWrite`]:
/// https://docs.rs/futures-preview/0.3.0-alpha.17/futures/io/trait.AsyncWrite.html
pub trait Write {
    /// Writes some bytes into the byte stream.
    ///
    /// Returns the number of bytes written from the start of the buffer.
    ///
    /// If the return value is `Ok(n)` then it must be guaranteed that `0 <= n <= buf.len()`. A
    /// return value of `0` typically means that the underlying object is no longer able to accept
    /// bytes and will likely not be able to in the future as well, or that the buffer provided is
    /// empty.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::{fs::File, prelude::*};
    ///
    /// let mut f = File::create("a.txt").await?;
    ///
    /// let n = f.write(b"hello world").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> ret!('a, WriteFuture, io::Result<usize>)
    where
        Self: Unpin;

    /// Flushes the stream to ensure that all buffered contents reach their destination.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::{fs::File, prelude::*};
    ///
    /// let mut f = File::create("a.txt").await?;
    ///
    /// f.write_all(b"hello world").await?;
    /// f.flush().await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    fn flush(&mut self) -> ret!('_, FlushFuture, io::Result<()>)
    where
        Self: Unpin;

    /// Like [`write`], except that it writes from a slice of buffers.
    ///
    /// Data is copied from each buffer in order, with the final buffer read from possibly being
    /// only partially consumed. This method must behave as a call to [`write`] with the buffers
    /// concatenated would.
    ///
    /// The default implementation calls [`write`] with either the first nonempty buffer provided,
    /// or an empty one if none exists.
    ///
    /// [`write`]: #tymethod.write
    fn write_vectored<'a>(
        &'a mut self,
        bufs: &'a [IoSlice<'a>],
    ) -> ret!('a, WriteVectoredFuture, io::Result<usize>)
    where
        Self: Unpin,
    {
        WriteVectoredFuture { writer: self, bufs }
    }

    /// Writes an entire buffer into the byte stream.
    ///
    /// This method will continuously call [`write`] until there is no more data to be written or
    /// an error is returned. This method will not return until the entire buffer has been
    /// successfully written or such an error occurs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::{fs::File, prelude::*};
    ///
    /// let mut f = File::create("a.txt").await?;
    ///
    /// f.write_all(b"hello world").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> ret!('a, WriteAllFuture, io::Result<()>)
    where
        Self: Unpin,
    {
        WriteAllFuture { writer: self, buf }
    }
}

impl<T: AsyncWrite + Unpin + ?Sized> Write for T {
    fn write<'a>(&'a mut self, buf: &'a [u8]) -> ret!('a, WriteFuture, io::Result<usize>) {
        WriteFuture { writer: self, buf }
    }

    fn flush(&mut self) -> ret!('_, FlushFuture, io::Result<()>) {
        FlushFuture { writer: self }
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct WriteFuture<'a, T: Unpin + ?Sized> {
    writer: &'a mut T,
    buf: &'a [u8],
}

impl<T: AsyncWrite + Unpin + ?Sized> Future for WriteFuture<'_, T> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let buf = self.buf;
        Pin::new(&mut *self.writer).poll_write(cx, buf)
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct FlushFuture<'a, T: Unpin + ?Sized> {
    writer: &'a mut T,
}

impl<T: AsyncWrite + Unpin + ?Sized> Future for FlushFuture<'_, T> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.writer).poll_flush(cx)
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct WriteVectoredFuture<'a, T: Unpin + ?Sized> {
    writer: &'a mut T,
    bufs: &'a [IoSlice<'a>],
}

impl<T: AsyncWrite + Unpin + ?Sized> Future for WriteVectoredFuture<'_, T> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let bufs = self.bufs;
        Pin::new(&mut *self.writer).poll_write_vectored(cx, bufs)
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct WriteAllFuture<'a, T: Unpin + ?Sized> {
    writer: &'a mut T,
    buf: &'a [u8],
}

impl<T: AsyncWrite + Unpin + ?Sized> Future for WriteAllFuture<'_, T> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self { writer, buf } = &mut *self;

        while !buf.is_empty() {
            let n = futures::ready!(Pin::new(&mut **writer).poll_write(cx, buf))?;
            let (_, rest) = mem::replace(buf, &[]).split_at(n);
            *buf = rest;

            if n == 0 {
                return Poll::Ready(Err(io::ErrorKind::WriteZero.into()));
            }
        }

        Poll::Ready(Ok(()))
    }
}
