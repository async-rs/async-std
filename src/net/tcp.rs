use std::io::{self, IoSlice, IoSliceMut};
use std::mem;
use std::net::{self, SocketAddr, ToSocketAddrs};
use std::pin::Pin;
use std::task::{Context, Poll};

use cfg_if::cfg_if;
use futures::{prelude::*, ready};
use futures::stream::FusedStream;

use crate::net::driver::IoHandle;

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
/// [`AsyncRead`]: https://docs.rs/futures-preview/0.3.0-alpha.13/futures/io/trait.AsyncRead.html
/// [`AsyncWrite`]: https://docs.rs/futures-preview/0.3.0-alpha.13/futures/io/trait.AsyncRead.html
/// [`futures::io`]: https://docs.rs/futures-preview/0.3.0-alpha.13/futures/io
/// [`shutdown`]: struct.TcpStream.html#method.shutdown
/// [`std::net::TcpStream`]: https://doc.rust-lang.org/std/net/struct.TcpStream.html
///
/// ## Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::{net::TcpStream, prelude::*};
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
    io_handle: IoHandle<mio::net::TcpStream>,

    #[cfg(unix)]
    raw_fd: std::os::unix::io::RawFd,
    // #[cfg(windows)]
    // raw_socket: std::os::windows::io::RawSocket,
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
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpStream;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:0").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn connect<A: ToSocketAddrs>(addrs: A) -> io::Result<TcpStream> {
        enum State {
            Waiting(TcpStream),
            Error(io::Error),
            Done,
        }

        let mut last_err = None;

        for addr in addrs.to_socket_addrs()? {
            let mut state = {
                match mio::net::TcpStream::connect(&addr) {
                    Ok(mio_stream) => {
                        #[cfg(unix)]
                        let stream = TcpStream {
                            raw_fd: mio_stream.as_raw_fd(),
                            io_handle: IoHandle::new(mio_stream),
                        };

                        #[cfg(windows)]
                        let stream = TcpStream {
                            // raw_socket: mio_stream.as_raw_socket(),
                            io_handle: IoHandle::new(mio_stream),
                        };

                        State::Waiting(stream)
                    }
                    Err(err) => State::Error(err),
                }
            };

            let res = future::poll_fn(|cx| {
                match mem::replace(&mut state, State::Done) {
                    State::Waiting(stream) => {
                        // Once we've connected, wait for the stream to be writable as that's when
                        // the actual connection has been initiated. Once we're writable we check
                        // for `take_socket_error` to see if the connect actually hit an error or
                        // not.
                        //
                        // If all that succeeded then we ship everything on up.
                        if let Poll::Pending = stream.io_handle.poll_writable(cx)? {
                            state = State::Waiting(stream);
                            return Poll::Pending;
                        }

                        if let Some(err) = stream.io_handle.get_ref().take_error()? {
                            return Poll::Ready(Err(err));
                        }

                        Poll::Ready(Ok(stream))
                    }
                    State::Error(err) => Poll::Ready(Err(err)),
                    State::Done => panic!("`TcpStream::connect()` future polled after completion"),
                }
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

    /// Returns the local address that this stream is connected to.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
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
        self.io_handle.get_ref().local_addr()
    }

    /// Returns the remote address that this stream is connected to.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
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
        self.io_handle.get_ref().peer_addr()
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
    /// # #![feature(async_await)]
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
        self.io_handle.get_ref().ttl()
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent
    /// from this socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
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
        self.io_handle.get_ref().set_ttl(ttl)
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
    /// # #![feature(async_await)]
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
        let res = future::poll_fn(|cx| {
            ready!(self.io_handle.poll_readable(cx)?);
            match self.io_handle.get_ref().peek(buf) {
                Ok(len) => Poll::Ready(Ok(len)),
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.io_handle.clear_readable(cx)?;
                    Poll::Pending
                }
                Err(e) => Poll::Ready(Err(e)),
            }
        })
        .await?;
        Ok(res)
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
    /// # #![feature(async_await)]
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
        self.io_handle.get_ref().nodelay()
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
    /// # #![feature(async_await)]
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
        self.io_handle.get_ref().set_nodelay(nodelay)
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
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpStream;
    /// use std::net::Shutdown;
    ///
    /// let stream = TcpStream::connect("127.0.0.1:8080").await?;
    /// stream.shutdown(Shutdown::Both)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn shutdown(&self, how: std::net::Shutdown) -> std::io::Result<()> {
        self.io_handle.get_ref().shutdown(how)
    }
}

impl futures::io::AsyncRead for TcpStream {
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

impl futures::io::AsyncRead for &TcpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &self.io_handle).poll_read(cx, buf)
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &self.io_handle).poll_read_vectored(cx, bufs)
    }
}

impl futures::io::AsyncWrite for TcpStream {
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

impl futures::io::AsyncWrite for &TcpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &self.io_handle).poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &self.io_handle).poll_write_vectored(cx, bufs)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &self.io_handle).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &self.io_handle).poll_close(cx)
    }
}

/// A TCP socket server, listening for connections.
///
/// After creating a `TcpListener` by [`bind`]ing it to a socket address, it listens for incoming
/// TCP connections. These can be accepted by awaiting elements from the async stream of
/// [`incoming`] connections.
///
/// The socket will be closed when the value is dropped.
///
/// The Transmission Control Protocol is specified in [IETF RFC 793].
///
/// This type is an async version of [`std::net::TcpListener`].
///
/// [`bind`]: #method.bind
/// [`incoming`]: #method.incoming
/// [IETF RFC 793]: https://tools.ietf.org/html/rfc793
/// [`std::net::TcpListener`]: https://doc.rust-lang.org/std/net/struct.TcpListener.html
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::{io, net::TcpListener, prelude::*};
///
/// let listener = TcpListener::bind("127.0.0.1:8080").await?;
/// let mut incoming = listener.incoming();
///
/// while let Some(stream) = incoming.next().await {
///     let stream = stream?;
///     let (reader, writer) = &mut (&stream, &stream);
///     io::copy(reader, writer).await?;
/// }
/// #
/// # Ok(()) }) }
/// ```
#[derive(Debug)]
pub struct TcpListener {
    io_handle: IoHandle<mio::net::TcpListener>,

    #[cfg(unix)]
    raw_fd: std::os::unix::io::RawFd,
    // #[cfg(windows)]
    // raw_socket: std::os::windows::io::RawSocket,
}

impl TcpListener {
    /// Creates a new `TcpListener` which will be bound to the specified address.
    ///
    /// The returned listener is ready for accepting connections.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port to this listener.
    /// The port allocated can be queried via the [`local_addr`] method.
    ///
    /// # Examples
    /// Create a TCP listener bound to 127.0.0.1:0:
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpListener;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    ///
    /// [`local_addr`]: #method.local_addr
    pub async fn bind<A: ToSocketAddrs>(addrs: A) -> io::Result<TcpListener> {
        let mut last_err = None;

        for addr in addrs.to_socket_addrs()? {
            match mio::net::TcpListener::bind(&addr) {
                Ok(mio_listener) => {
                    #[cfg(unix)]
                    let listener = TcpListener {
                        raw_fd: mio_listener.as_raw_fd(),
                        io_handle: IoHandle::new(mio_listener),
                    };

                    #[cfg(windows)]
                    let listener = TcpListener {
                        // raw_socket: mio_listener.as_raw_socket(),
                        io_handle: IoHandle::new(mio_listener),
                    };
                    return Ok(listener);
                }
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

    /// Accepts a new incoming connection to this listener.
    ///
    /// When a connection is established, the corresponding stream and address will be returned.
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpListener;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0").await?;
    /// let (stream, addr) = listener.accept().await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        future::poll_fn(|cx| {
            ready!(self.io_handle.poll_readable(cx)?);

            match self.io_handle.get_ref().accept_std() {
                Ok((io, addr)) => {
                    let mio_stream = mio::net::TcpStream::from_stream(io)?;

                    #[cfg(unix)]
                    let stream = TcpStream {
                        raw_fd: mio_stream.as_raw_fd(),
                        io_handle: IoHandle::new(mio_stream),
                    };

                    #[cfg(windows)]
                    let stream = TcpStream {
                        // raw_socket: mio_stream.as_raw_socket(),
                        io_handle: IoHandle::new(mio_stream),
                    };

                    Poll::Ready(Ok((stream, addr)))
                }
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.io_handle.clear_readable(cx)?;
                    Poll::Pending
                }
                Err(err) => Poll::Ready(Err(err)),
            }
        })
        .await
    }

    /// Returns a stream of incoming connections.
    ///
    /// Iterating over this stream is equivalent to calling [`accept`] in a loop. The stream of
    /// connections is infinite, i.e awaiting the next connection will never result in [`None`].
    ///
    /// [`accept`]: #method.accept
    /// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::{net::TcpListener, prelude::*};
    ///
    /// let listener = TcpListener::bind("127.0.0.1:0").await?;
    /// let mut incoming = listener.incoming();
    ///
    /// while let Some(stream) = incoming.next().await {
    ///     let mut stream = stream?;
    ///     stream.write_all(b"hello world").await?;
    /// }
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn incoming(&self) -> Incoming<'_> {
        Incoming(self)
    }

    /// Returns the local address that this listener is bound to.
    ///
    /// This can be useful, for example, to identify when binding to port 0 which port was assigned
    /// by the OS.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpListener;
    ///
    /// let listener = TcpListener::bind("127.0.0.1:8080").await?;
    /// let addr = listener.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.io_handle.get_ref().local_addr()
    }
}

/// A stream of incoming TCP connections.
///
/// This stream is infinite, i.e awaiting the next connection will never result in [`None`]. It is
/// created by the [`incoming`] method on [`TcpListener`].
///
/// This type is an async version of [`std::net::Incoming`].
///
/// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
/// [`incoming`]: struct.TcpListener.html#method.incoming
/// [`TcpListener`]: struct.TcpListener.html
/// [`std::net::Incoming`]: https://doc.rust-lang.org/std/net/struct.Incoming.html
#[derive(Debug)]
pub struct Incoming<'a>(&'a TcpListener);

impl<'a> futures::Stream for Incoming<'a> {
    type Item = io::Result<TcpStream>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let future = self.0.accept();
        pin_utils::pin_mut!(future);

        let (socket, _) = ready!(future.poll(cx))?;
        Poll::Ready(Some(Ok(socket)))
    }
}

impl<'a> FusedStream for Incoming<'a> {
    fn is_terminated(&self) -> bool { false }
}

impl From<net::TcpStream> for TcpStream {
    /// Converts a `std::net::TcpStream` into its asynchronous equivalent.
    fn from(stream: net::TcpStream) -> TcpStream {
        let mio_stream = mio::net::TcpStream::from_stream(stream).unwrap();

        #[cfg(unix)]
        let stream = TcpStream {
            raw_fd: mio_stream.as_raw_fd(),
            io_handle: IoHandle::new(mio_stream),
        };

        #[cfg(windows)]
        let stream = TcpStream {
            // raw_socket: mio_stream.as_raw_socket(),
            io_handle: IoHandle::new(mio_stream),
        };

        stream
    }
}

impl From<net::TcpListener> for TcpListener {
    /// Converts a `std::net::TcpListener` into its asynchronous equivalent.
    fn from(listener: net::TcpListener) -> TcpListener {
        let mio_listener = mio::net::TcpListener::from_std(listener).unwrap();

        #[cfg(unix)]
        let listener = TcpListener {
            raw_fd: mio_listener.as_raw_fd(),
            io_handle: IoHandle::new(mio_listener),
        };

        #[cfg(windows)]
        let listener = TcpListener {
            // raw_socket: mio_listener.as_raw_socket(),
            io_handle: IoHandle::new(mio_listener),
        };

        listener
    }
}

cfg_if! {
    if #[cfg(feature = "docs.rs")] {
        use crate::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
        // use crate::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle};
    } else if #[cfg(unix)] {
        use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
    } else if #[cfg(windows)] {
        // use std::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, RawHandle};
    }
}

#[cfg_attr(feature = "docs.rs", doc(cfg(unix)))]
cfg_if! {
    if #[cfg(any(unix, feature = "docs.rs"))] {
        impl AsRawFd for TcpListener {
            fn as_raw_fd(&self) -> RawFd {
                self.raw_fd
            }
        }

        impl FromRawFd for TcpListener {
            unsafe fn from_raw_fd(fd: RawFd) -> TcpListener {
                net::TcpListener::from_raw_fd(fd).into()
            }
        }

        impl IntoRawFd for TcpListener {
            fn into_raw_fd(self) -> RawFd {
                self.raw_fd
            }
        }

        impl AsRawFd for TcpStream {
            fn as_raw_fd(&self) -> RawFd {
                self.raw_fd
            }
        }

        impl FromRawFd for TcpStream {
            unsafe fn from_raw_fd(fd: RawFd) -> TcpStream {
                net::TcpStream::from_raw_fd(fd).into()
            }
        }

        impl IntoRawFd for TcpStream {
            fn into_raw_fd(self) -> RawFd {
                self.raw_fd
            }
        }
    }
}

#[cfg_attr(feature = "docs.rs", doc(cfg(windows)))]
cfg_if! {
    if #[cfg(any(windows, feature = "docs.rs"))] {
        // impl AsRawSocket for TcpListener {
        //     fn as_raw_socket(&self) -> RawSocket {
        //         self.raw_socket
        //     }
        // }
        //
        // impl FromRawSocket for TcpListener {
        //     unsafe fn from_raw_socket(handle: RawSocket) -> TcpListener {
        //         net::TcpListener::from_raw_socket(handle).try_into().unwrap()
        //     }
        // }
        //
        // impl IntoRawSocket for TcpListener {
        //     fn into_raw_socket(self) -> RawSocket {
        //         self.raw_socket
        //     }
        // }
        //
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
