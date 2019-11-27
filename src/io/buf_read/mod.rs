mod lines;
mod read_line;
mod read_until;
mod split;

pub use lines::Lines;
pub use split::Split;

use read_line::ReadLineFuture;
use read_until::ReadUntilFuture;

use std::mem;
use std::pin::Pin;

use crate::io;
use crate::task::{Context, Poll};

extension_trait! {
    use std::ops::{Deref, DerefMut};

    #[doc = r#"
        Allows reading from a buffered byte stream.

        This trait is a re-export of [`futures::io::AsyncBufRead`] and is an async version of
        [`std::io::BufRead`].

        The [provided methods] do not really exist in the trait itself, but they become
        available when [`BufReadExt`] from the [prelude] is imported:

        ```
        # #[allow(unused_imports)]
        use async_std::prelude::*;
        ```

        [`std::io::BufRead`]: https://doc.rust-lang.org/std/io/trait.BufRead.html
        [`futures::io::AsyncBufRead`]:
        https://docs.rs/futures/0.3/futures/io/trait.AsyncBufRead.html
        [provided methods]: #provided-methods
        [`BufReadExt`]: ../io/prelude/trait.BufReadExt.html
        [prelude]: ../prelude/index.html
    "#]
    pub trait BufRead {
        #[doc = r#"
            Returns the contents of the internal buffer, filling it with more data from the
            inner reader if it is empty.

            This function is a lower-level call. It needs to be paired with the [`consume`]
            method to function properly. When calling this method, none of the contents will be
            "read" in the sense that later calling `read` may return the same contents. As
            such, [`consume`] must be called with the number of bytes that are consumed from
            this buffer to ensure that the bytes are never returned twice.

            [`consume`]: #tymethod.consume

            An empty buffer returned indicates that the stream has reached EOF.
        "#]
        // TODO: write a proper doctest with `consume`
        fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&[u8]>>;

        #[doc = r#"
            Tells this buffer that `amt` bytes have been consumed from the buffer, so they
            should no longer be returned in calls to `read`.
        "#]
        fn consume(self: Pin<&mut Self>, amt: usize);
    }

    #[doc = r#"
        Extension methods for [`BufRead`].

        [`BufRead`]: ../trait.BufRead.html
    "#]
    pub trait BufReadExt: futures_io::AsyncBufRead {
        #[doc = r#"
            Reads all bytes into `buf` until the delimiter `byte` or EOF is reached.

            This function will read bytes from the underlying stream until the delimiter or EOF
            is found. Once found, all bytes up to, and including, the delimiter (if found) will
            be appended to `buf`.

            If successful, this function will return the total number of bytes read.

            # Examples

            ```no_run
            # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            #
            use async_std::fs::File;
            use async_std::io::BufReader;
            use async_std::prelude::*;

            let mut file = BufReader::new(File::open("a.txt").await?);

            let mut buf = Vec::with_capacity(1024);
            let n = file.read_until(b'\n', &mut buf).await?;
            #
            # Ok(()) }) }
            ```

            Multiple successful calls to `read_until` append all bytes up to and including to
            `buf`:
            ```
            # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            #
            use async_std::io::BufReader;
            use async_std::prelude::*;

            let from: &[u8] = b"append\nexample\n";
            let mut reader = BufReader::new(from);
            let mut buf = vec![];

            let mut size = reader.read_until(b'\n', &mut buf).await?;
            assert_eq!(size, 7);
            assert_eq!(buf, b"append\n");

            size += reader.read_until(b'\n', &mut buf).await?;
            assert_eq!(size, from.len());

            assert_eq!(buf, from);
            #
            # Ok(()) }) }
            ```
        "#]
        fn read_until<'a>(
            &'a mut self,
            byte: u8,
            buf: &'a mut Vec<u8>,
        ) -> impl Future<Output = usize> + 'a [ReadUntilFuture<'a, Self>]
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

        #[doc = r#"
            Reads all bytes and appends them into `buf` until a newline (the 0xA byte) is
            reached.

            This function will read bytes from the underlying stream until the newline
            delimiter (the 0xA byte) or EOF is found. Once found, all bytes up to, and
            including, the delimiter (if found) will be appended to `buf`.

            If successful, this function will return the total number of bytes read.

            If this function returns `Ok(0)`, the stream has reached EOF.

            # Errors

            This function has the same error semantics as [`read_until`] and will also return
            an error if the read bytes are not valid UTF-8. If an I/O error is encountered then
            `buf` may contain some bytes already read in the event that all data read so far
            was valid UTF-8.

            [`read_until`]: #method.read_until

            # Examples

            ```no_run
            # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            #
            use async_std::fs::File;
            use async_std::io::BufReader;
            use async_std::prelude::*;

            let mut file = BufReader::new(File::open("a.txt").await?);

            let mut buf = String::new();
            file.read_line(&mut buf).await?;
            #
            # Ok(()) }) }
            ```
        "#]
        fn read_line<'a>(
            &'a mut self,
            buf: &'a mut String,
        ) -> impl Future<Output = io::Result<usize>> + 'a [ReadLineFuture<'a, Self>]
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

        #[doc = r#"
            Returns a stream over the lines of this byte stream.

            The stream returned from this function will yield instances of
            [`io::Result`]`<`[`String`]`>`. Each string returned will *not* have a newline byte
            (the 0xA byte) or CRLF (0xD, 0xA bytes) at the end.

            [`io::Result`]: type.Result.html
            [`String`]: https://doc.rust-lang.org/std/string/struct.String.html

            # Examples

            ```no_run
            # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            #
            use async_std::fs::File;
            use async_std::io::BufReader;
            use async_std::prelude::*;

            let file = File::open("a.txt").await?;
            let mut lines = BufReader::new(file).lines();
            let mut count = 0;

            while let Some(line) = lines.next().await {
                line?;
                count += 1;
            }
            #
            # Ok(()) }) }
            ```
        "#]
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

        #[doc = r#"
            Returns a stream over the contents of this reader split on the byte `byte`.

            The stream returned from this function will return instances of
            [`io::Result`]`<`[`Vec<u8>`]`>`. Each vector returned will *not* have
            the delimiter byte at the end.

            This function will yield errors whenever [`read_until`] would have
            also yielded an error.

            [`io::Result`]: type.Result.html
            [`Vec<u8>`]: ../vec/struct.Vec.html
            [`read_until`]: #method.read_until

            # Examples

            [`std::io::Cursor`][`Cursor`] is a type that implements `BufRead`. In
            this example, we use [`Cursor`] to iterate over all hyphen delimited
            segments in a byte slice

            [`Cursor`]: struct.Cursor.html

            ```
            # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
            #
            use async_std::prelude::*;
            use async_std::io;

            let cursor = io::Cursor::new(b"lorem-ipsum-dolor");

            let mut split_iter = cursor.split(b'-').map(|l| l.unwrap());
            assert_eq!(split_iter.next().await, Some(b"lorem".to_vec()));
            assert_eq!(split_iter.next().await, Some(b"ipsum".to_vec()));
            assert_eq!(split_iter.next().await, Some(b"dolor".to_vec()));
            assert_eq!(split_iter.next().await, None);
            #
            # Ok(()) }) }
            ```
        "#]
        fn split(self, byte: u8) -> Split<Self>
        where
            Self: Sized,
        {
            Split {
                reader: self,
                buf: Vec::new(),
                delim: byte,
                read: 0,
            }
        }
    }

    impl<T: BufRead + Unpin + ?Sized> BufRead for Box<T> {
        fn poll_fill_buf(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<io::Result<&[u8]>> {
            unreachable!("this impl only appears in the rendered docs")
        }

        fn consume(self: Pin<&mut Self>, amt: usize) {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl<T: BufRead + Unpin + ?Sized> BufRead for &mut T {
        fn poll_fill_buf(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<io::Result<&[u8]>> {
            unreachable!("this impl only appears in the rendered docs")
        }

        fn consume(self: Pin<&mut Self>, amt: usize) {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl<P> BufRead for Pin<P>
    where
        P: DerefMut + Unpin,
        <P as Deref>::Target: BufRead,
    {
        fn poll_fill_buf(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<io::Result<&[u8]>> {
            unreachable!("this impl only appears in the rendered docs")
        }

        fn consume(self: Pin<&mut Self>, amt: usize) {
            unreachable!("this impl only appears in the rendered docs")
        }
    }

    impl BufRead for &[u8] {
        fn poll_fill_buf(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<io::Result<&[u8]>> {
            unreachable!()
        }

        fn consume(self: Pin<&mut Self>, amt: usize) {
            unreachable!("this impl only appears in the rendered docs")
        }
    }
}

pub fn read_until_internal<R: BufReadExt + ?Sized>(
    mut reader: Pin<&mut R>,
    cx: &mut Context<'_>,
    byte: u8,
    buf: &mut Vec<u8>,
    read: &mut usize,
) -> Poll<io::Result<usize>> {
    loop {
        let (done, used) = {
            let available = futures_core::ready!(reader.as_mut().poll_fill_buf(cx))?;
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
