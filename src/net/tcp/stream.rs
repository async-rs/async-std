use std::io::{IoSlice, IoSliceMut};
use std::net::SocketAddr;
use std::pin::Pin;

use cfg_if::cfg_if;

use crate::io::{self, Read, Write};
use crate::net::ToSocketAddrs;
use crate::task::{Context, Poll};

cfg_if! {
    if #[cfg(not(target_os = "unknown"))] {
        use std::io::{Read as _, Write as _};

        use crate::future;
        use crate::net::driver::Watcher;
        use crate::task::blocking;
    }
}

/// A TCP stream between a local and a remote socket.
///
/// A `TcpStream` can either be created by connecting to an endpoint, via the [`connect`] method,
/// or by [accepting] a connection from a [listener].  It can be read or written to using the
/// [`AsyncRead`], [`AsyncWrite`], and related extension traits in [`futures::io`].
///
/// The connection will be closed when the value is dropped. The reading and writing portions of
/// the connection can also be shut down individually with the [`shutdown`] method.
///
/// This type is an async version of [`std::net::TcpStream`].
///
/// [`connect`]: struct.TcpStream.html#method.connect
/// [accepting]: struct.TcpListener.html#method.accept
/// [listener]: struct.TcpListener.html
/// [`AsyncRead`]: https://docs.rs/futures-preview/0.3.0-alpha.17/futures/io/trait.AsyncRead.html
/// [`AsyncWrite`]: https://docs.rs/futures-preview/0.3.0-alpha.17/futures/io/trait.AsyncWrite.html
/// [`futures::io`]: https://docs.rs/futures-preview/0.3.0-alpha.17/futures/io/index.html
/// [`shutdown`]: struct.TcpStream.html#method.shutdown
/// [`std::net::TcpStream`]: https://doc.rust-lang.org/std/net/struct.TcpStream.html
///
/// ## Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::net::TcpStream;
/// use async_std::prelude::*;
///
/// let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
/// stream.write_all(b"hello world").await?;
///
/// let mut buf = vec![0u8; 1024];
/// let n = stream.read(&mut buf).await?;
/// #
/// # Ok(()) }) }
/// ```
#[derive(Debug)]
pub struct TcpStream {
    pub(super) inner: Inner,
}

cfg_if! {
    if #[cfg(not(target_os = "unknown"))] {
        #[derive(Debug)]
        pub(super) struct Inner {
            pub(super) watcher: Watcher<mio::net::TcpStream>,
        }
    } else {
        #[derive(Debug)]
        pub(super) struct Inner;
    }
}

impl TcpStream {
    /// Creates a new TCP stream connected to the specified address.
    ///
    /// This method will create a new TCP socket and attempt to connect it to the `addr`
    /// provided. The [returned future] will be resolved once the stream has successfully
    /// connected, or it will return an error if one occurs.
    ///
    /// [returned future]: struct.Connect.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:0").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn connect<A: ToSocketAddrs>(addrs: A) -> io::Result<TcpStream> {
        connect(addrs).await
    }

    /// Returns the local address that this stream is connected to.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// let addr = stream.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        local_addr(self)
    }

    /// Returns the remote address that this stream is connected to.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// let peer = stream.peer_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        peer_addr(self)
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_ttl`].
    ///
    /// [`set_ttl`]: #method.set_ttl
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_ttl(100)?;
    /// assert_eq!(stream.ttl()?, 100);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn ttl(&self) -> io::Result<u32> {
        ttl(self)
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent
    /// from this socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_ttl(100)?;
    /// assert_eq!(stream.ttl()?, 100);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        set_ttl(self, ttl)
    }

    /// Receives data on the socket from the remote address to which it is connected, without
    /// removing that data from the queue.
    ///
    /// On success, returns the number of bytes peeked.
    ///
    /// Successive calls return the same data. This is accomplished by passing `MSG_PEEK` as a flag
    /// to the underlying `recv` system call.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8000").await?;
    ///
    /// let mut buf = vec![0; 1024];
    /// let n = stream.peek(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        peek(self, buf).await
    }

    /// Gets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// For more information about this option, see [`set_nodelay`].
    ///
    /// [`set_nodelay`]: #method.set_nodelay
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_nodelay(true)?;
    /// assert_eq!(stream.nodelay()?, true);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn nodelay(&self) -> io::Result<bool> {
        nodelay(self)
    }

    /// Sets the value of the `TCP_NODELAY` option on this socket.
    ///
    /// If set, this option disables the Nagle algorithm. This means that
    /// segments are always sent as soon as possible, even if there is only a
    /// small amount of data. When not set, data is buffered until there is a
    /// sufficient amount to send out, thereby avoiding the frequent sending of
    /// small packets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    ///
    /// stream.set_nodelay(true)?;
    /// assert_eq!(stream.nodelay()?, true);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        set_nodelay(self, nodelay)
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This method will cause all pending and future I/O on the specified portions to return
    /// immediately with an appropriate value (see the documentation of [`Shutdown`]).
    ///
    /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use std::net::Shutdown;
    ///
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// stream.shutdown(Shutdown::Both)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn shutdown(&self, how: std::net::Shutdown) -> std::io::Result<()> {
        shutdown(self, how)
    }
}

impl Read for TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_read(cx, buf)
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_read_vectored(cx, bufs)
    }
}

impl Write for TcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_write_vectored(cx, bufs)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_close(cx)
    }
}

cfg_if! {
    if #[cfg(not(target_os = "unknown"))] {
        impl Read for &TcpStream {
            fn poll_read(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &mut [u8],
            ) -> Poll<io::Result<usize>> {
                self.inner.watcher.poll_read_with(cx, |mut inner| inner.read(buf))
            }
        }

        impl Write for &TcpStream {
            fn poll_write(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &[u8],
            ) -> Poll<io::Result<usize>> {
                self.inner.watcher
                    .poll_write_with(cx, |mut inner| inner.write(buf))
            }

            fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
                self.inner.watcher.poll_write_with(cx, |mut inner| inner.flush())
            }

            fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
                Poll::Ready(Ok(()))
            }
        }

    } else {
        impl Read for &TcpStream {
            fn poll_read(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
                _: &mut [u8],
            ) -> Poll<io::Result<usize>> {
                unreachable!()
            }
        }

        impl Write for &TcpStream {
            fn poll_write(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
                _: &[u8],
            ) -> Poll<io::Result<usize>> {
                unreachable!()
            }

            fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
                unreachable!()
            }

            fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
                unreachable!()
            }
        }
    }
}

impl From<std::net::TcpStream> for TcpStream {
    /// Converts a `std::net::TcpStream` into its asynchronous equivalent.
    fn from(stream: std::net::TcpStream) -> TcpStream {
        from(stream)
    }
}

cfg_if! {
    if #[cfg(feature = "docs")] {
        use crate::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
        // use crate::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle};
    } else if #[cfg(unix)] {
        use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
    } else if #[cfg(windows)] {
        // use std::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle};
    }
}

#[cfg_attr(feature = "docs", doc(cfg(unix)))]
cfg_if! {
    if #[cfg(any(unix, feature = "docs"))] {
        impl AsRawFd for TcpStream {
            fn as_raw_fd(&self) -> RawFd {
                self.inner.watcher.get_ref().as_raw_fd()
            }
        }

        impl FromRawFd for TcpStream {
            unsafe fn from_raw_fd(fd: RawFd) -> TcpStream {
                std::net::TcpStream::from_raw_fd(fd).into()
            }
        }

        impl IntoRawFd for TcpStream {
            fn into_raw_fd(self) -> RawFd {
                self.inner.watcher.into_inner().into_raw_fd()
            }
        }
    }
}

#[cfg_attr(feature = "docs", doc(cfg(windows)))]
cfg_if! {
    if #[cfg(any(windows, feature = "docs"))] {
        // impl AsRawSocket for TcpStream {
        //     fn as_raw_socket(&self) -> RawSocket {
        //         self.raw_socket
        //     }
        // }
        //
        // impl FromRawSocket for TcpStream {
        //     unsafe fn from_raw_socket(handle: RawSocket) -> TcpStream {
        //         net::TcpStream::from_raw_socket(handle).try_into().unwrap()
        //     }
        // }
        //
        // impl IntoRawSocket for TcpListener {
        //     fn into_raw_socket(self) -> RawSocket {
        //         self.raw_socket
        //     }
        // }
    }
}

cfg_if! {
    if #[cfg(not(target_os = "unknown"))] {
        async fn connect<A: ToSocketAddrs>(addrs: A) -> io::Result<TcpStream> {
            let mut last_err = None;

            for addr in addrs.to_socket_addrs().await? {
                let res = blocking::spawn(async move {
                    let std_stream = std::net::TcpStream::connect(addr)?;
                    let mio_stream = mio::net::TcpStream::from_stream(std_stream)?;
                    Ok(TcpStream {
                        inner: Inner {
                            watcher: Watcher::new(mio_stream),
                        },
                    })
                })
                .await;

                match res {
                    Ok(stream) => return Ok(stream),
                    Err(err) => last_err = Some(err),
                }
            }

            Err(last_err.unwrap_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "could not resolve to any addresses",
                )
            }))
        }

        fn local_addr(socket: &TcpStream) -> io::Result<SocketAddr> {
            socket.inner.watcher.get_ref().local_addr()
        }

        fn peer_addr(socket: &TcpStream) -> io::Result<SocketAddr> {
            socket.inner.watcher.get_ref().peer_addr()
        }

        fn ttl(socket: &TcpStream) -> io::Result<u32> {
            socket.inner.watcher.get_ref().ttl()
        }

        fn set_ttl(socket: &TcpStream, ttl: u32) -> io::Result<()> {
            socket.inner.watcher.get_ref().set_ttl(ttl)
        }

        async fn peek(socket: &TcpStream, buf: &mut [u8]) -> io::Result<usize> {
            future::poll_fn(|cx| socket.inner.watcher.poll_read_with(cx, |inner| inner.peek(buf))).await
        }

        fn nodelay(socket: &TcpStream) -> io::Result<bool> {
            socket.inner.watcher.get_ref().nodelay()
        }

        fn set_nodelay(socket: &TcpStream, nodelay: bool) -> io::Result<()> {
            socket.inner.watcher.get_ref().set_nodelay(nodelay)
        }

        fn shutdown(socket: &TcpStream, how: std::net::Shutdown) -> std::io::Result<()> {
            socket.inner.watcher.get_ref().shutdown(how)
        }

        fn from(stream: std::net::TcpStream) -> TcpStream {
            let mio_stream = mio::net::TcpStream::from_stream(stream).unwrap();
            TcpStream {
                inner: Inner {
                    watcher: Watcher::new(mio_stream),
                },
            }
        }

    } else {
        async fn connect<A: ToSocketAddrs>(_: A) -> io::Result<TcpStream> {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "TCP sockets unsupported on this platform",
            ))
        }

        fn local_addr(_: &TcpStream) -> io::Result<SocketAddr> {
            unreachable!()
        }

        fn peer_addr(_: &TcpStream) -> io::Result<SocketAddr> {
            unreachable!()
        }

        fn ttl(_: &TcpStream) -> io::Result<u32> {
            unreachable!()
        }

        fn set_ttl(_: &TcpStream, _: u32) -> io::Result<()> {
            unreachable!()
        }

        async fn peek(_: &TcpStream, _: &mut [u8]) -> io::Result<usize> {
            unreachable!()
        }

        fn nodelay(_: &TcpStream) -> io::Result<bool> {
            unreachable!()
        }

        fn set_nodelay(_: &TcpStream, _: bool) -> io::Result<()> {
            unreachable!()
        }

        fn shutdown(_: &TcpStream, _: std::net::Shutdown) -> std::io::Result<()> {
            unreachable!()
        }

        fn from(_: std::net::TcpStream) -> TcpStream {
            // We can never successfully build a `std::net::TcpStream` on an unknown OS.
            unreachable!()
        }
    }
}
