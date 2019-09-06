mod flush;
mod write;
mod write_all;
mod write_vectored;

use flush::FlushFuture;
use write::WriteFuture;
use write_all::WriteAllFuture;
use write_vectored::WriteVectoredFuture;

use std::io;

use cfg_if::cfg_if;
use futures_io::AsyncWrite;

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
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::prelude::*;
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
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::prelude::*;
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
        bufs: &'a [io::IoSlice<'a>],
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
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::fs::File;
    /// use async_std::prelude::*;
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
