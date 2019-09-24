mod read;
mod read_exact;
mod read_to_end;
mod read_to_string;
mod read_vectored;

use read::ReadFuture;
use read_exact::ReadExactFuture;
use read_to_end::{read_to_end_internal, ReadToEndFuture};
use read_to_string::ReadToStringFuture;
use read_vectored::ReadVectoredFuture;

use std::mem;

use cfg_if::cfg_if;

use crate::io::IoSliceMut;

cfg_if! {
    if #[cfg(feature = "docs")] {
        use std::pin::Pin;
        use std::ops::{Deref, DerefMut};

        use crate::io;
        use crate::task::{Context, Poll};

        #[doc(hidden)]
        pub struct ImplFuture<'a, T>(std::marker::PhantomData<&'a T>);

        /// Allows reading from a byte stream.
        ///
        /// This trait is a re-export of [`futures::io::AsyncRead`] and is an async version of
        /// [`std::io::Read`].
        ///
        /// Methods other than [`poll_read`] and [`poll_read_vectored`] do not really exist in the
        /// trait itself, but they become available when the prelude is imported:
        ///
        /// ```
        /// # #[allow(unused_imports)]
        /// use async_std::prelude::*;
        /// ```
        ///
        /// [`std::io::Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
        /// [`futures::io::AsyncRead`]:
        /// https://docs.rs/futures-preview/0.3.0-alpha.17/futures/io/trait.AsyncRead.html
        /// [`poll_read`]: #tymethod.poll_read
        /// [`poll_read_vectored`]: #method.poll_read_vectored
        pub trait Read {
            /// Attempt to read from the `AsyncRead` into `buf`.
            fn poll_read(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &mut [u8],
            ) -> Poll<io::Result<usize>>;

            /// Attempt to read from the `AsyncRead` into `bufs` using vectored IO operations.
            fn poll_read_vectored(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                bufs: &mut [IoSliceMut<'_>],
            ) -> Poll<io::Result<usize>> {
                unreachable!()
            }

            /// Reads some bytes from the byte stream.
            ///
            /// Returns the number of bytes read from the start of the buffer.
            ///
            /// If the return value is `Ok(n)`, then it must be guaranteed that
            /// `0 <= n <= buf.len()`. A nonzero `n` value indicates that the buffer has been
            /// filled in with `n` bytes of data. If `n` is `0`, then it can indicate one of two
            /// scenarios:
            ///
            /// 1. This reader has reached its "end of file" and will likely no longer be able to
            ///    produce bytes. Note that this does not mean that the reader will always no
            ///    longer be able to produce bytes.
            /// 2. The buffer specified was 0 bytes in length.
            ///
            /// # Examples
            ///
            /// ```no_run
            /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            /// #
            /// use async_std::fs::File;
            /// use async_std::prelude::*;
            ///
            /// let mut file = File::open("a.txt").await?;
            ///
            /// let mut buf = vec![0; 1024];
            /// let n = file.read(&mut buf).await?;
            /// #
            /// # Ok(()) }) }
            /// ```
            fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ImplFuture<'a, io::Result<usize>>
            where
                Self: Unpin
            {
                unreachable!()
            }

            /// Like [`read`], except that it reads into a slice of buffers.
            ///
            /// Data is copied to fill each buffer in order, with the final buffer written to
            /// possibly being only partially filled. This method must behave as a single call to
            /// [`read`] with the buffers concatenated would.
            ///
            /// The default implementation calls [`read`] with either the first nonempty buffer
            /// provided, or an empty one if none exists.
            ///
            /// [`read`]: #tymethod.read
            fn read_vectored<'a>(
                &'a mut self,
                bufs: &'a mut [IoSliceMut<'a>],
            ) -> ImplFuture<'a, io::Result<usize>>
            where
                Self: Unpin,
            {
                unreachable!()
            }

            /// Reads all bytes from the byte stream.
            ///
            /// All bytes read from this stream will be appended to the specified buffer `buf`.
            /// This function will continuously call [`read`] to append more data to `buf` until
            /// [`read`] returns either `Ok(0)` or an error.
            ///
            /// If successful, this function will return the total number of bytes read.
            ///
            /// [`read`]: #tymethod.read
            ///
            /// # Examples
            ///
            /// ```no_run
            /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            /// #
            /// use async_std::fs::File;
            /// use async_std::prelude::*;
            ///
            /// let mut file = File::open("a.txt").await?;
            ///
            /// let mut buf = Vec::new();
            /// file.read_to_end(&mut buf).await?;
            /// #
            /// # Ok(()) }) }
            /// ```
            fn read_to_end<'a>(
                &'a mut self,
                buf: &'a mut Vec<u8>,
            ) -> ImplFuture<'a, io::Result<usize>>
            where
                Self: Unpin,
            {
                unreachable!()
            }

            /// Reads all bytes from the byte stream and appends them into a string.
            ///
            /// If successful, this function will return the number of bytes read.
            ///
            /// If the data in this stream is not valid UTF-8 then an error will be returned and
            /// `buf` will be left unmodified.
            ///
            /// # Examples
            ///
            /// ```no_run
            /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            /// #
            /// use async_std::fs::File;
            /// use async_std::prelude::*;
            ///
            /// let mut file = File::open("a.txt").await?;
            ///
            /// let mut buf = String::new();
            /// file.read_to_string(&mut buf).await?;
            /// #
            /// # Ok(()) }) }
            /// ```
            fn read_to_string<'a>(
                &'a mut self,
                buf: &'a mut String,
            ) -> ImplFuture<'a, io::Result<usize>>
            where
                Self: Unpin,
            {
                unreachable!()
            }

            /// Reads the exact number of bytes required to fill `buf`.
            ///
            /// This function reads as many bytes as necessary to completely fill the specified
            /// buffer `buf`.
            ///
            /// No guarantees are provided about the contents of `buf` when this function is
            /// called, implementations cannot rely on any property of the contents of `buf` being
            /// true. It is recommended that implementations only write data to `buf` instead of
            /// reading its contents.
            ///
            /// If this function encounters an "end of file" before completely filling the buffer,
            /// it returns an error of the kind [`ErrorKind::UnexpectedEof`].  The contents of
            /// `buf` are unspecified in this case.
            ///
            /// If any other read error is encountered then this function immediately returns. The
            /// contents of `buf` are unspecified in this case.
            ///
            /// If this function returns an error, it is unspecified how many bytes it has read,
            /// but it will never read more than would be necessary to completely fill the buffer.
            ///
            /// [`ErrorKind::UnexpectedEof`]: enum.ErrorKind.html#variant.UnexpectedEof
            ///
            /// # Examples
            ///
            /// ```no_run
            /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            /// #
            /// use async_std::fs::File;
            /// use async_std::prelude::*;
            ///
            /// let mut file = File::open("a.txt").await?;
            ///
            /// let mut buf = vec![0; 10];
            /// file.read_exact(&mut buf).await?;
            /// #
            /// # Ok(()) }) }
            /// ```
            fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> ImplFuture<'a, io::Result<()>>
            where
                Self: Unpin,
            {
                unreachable!()
            }
        }

        impl<T: Read + Unpin + ?Sized> Read for Box<T> {
            fn poll_read(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &mut [u8],
            ) -> Poll<io::Result<usize>> {
                unreachable!()
            }
        }

        impl<T: Read + Unpin + ?Sized> Read for &mut T {
            fn poll_read(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &mut [u8],
            ) -> Poll<io::Result<usize>> {
                unreachable!()
            }
        }

        impl<P> Read for Pin<P>
        where
            P: DerefMut + Unpin,
            <P as Deref>::Target: Read,
        {
            fn poll_read(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &mut [u8],
            ) -> Poll<io::Result<usize>> {
                unreachable!()
            }
        }

        impl Read for &[u8] {
            fn poll_read(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &mut [u8],
            ) -> Poll<io::Result<usize>> {
                unreachable!()
            }
        }
    } else {
        pub use futures_io::AsyncRead as Read;
    }
}

#[doc(hidden)]
pub trait ReadExt: futures_io::AsyncRead {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadFuture<'a, Self>
    where
        Self: Unpin,
    {
        ReadFuture { reader: self, buf }
    }

    fn read_vectored<'a>(
        &'a mut self,
        bufs: &'a mut [IoSliceMut<'a>],
    ) -> ReadVectoredFuture<'a, Self>
    where
        Self: Unpin,
    {
        ReadVectoredFuture { reader: self, bufs }
    }

    fn read_to_end<'a>(&'a mut self, buf: &'a mut Vec<u8>) -> ReadToEndFuture<'a, Self>
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

    fn read_to_string<'a>(&'a mut self, buf: &'a mut String) -> ReadToStringFuture<'a, Self>
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

    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> ReadExactFuture<'a, Self>
    where
        Self: Unpin,
    {
        ReadExactFuture { reader: self, buf }
    }
}

impl<T: futures_io::AsyncRead + ?Sized> ReadExt for T {}
