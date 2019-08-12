//! Unix-specific networking extensions.

use std::fmt;
use std::io;
use std::mem;
use std::net::Shutdown;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

use cfg_if::cfg_if;
use futures::{prelude::*, ready};
use mio_uds;

use crate::net::driver::IoHandle;
use crate::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use crate::task::blocking;

/// A Unix datagram socket.
///
/// After creating a `UnixDatagram` by [`bind`]ing it to a path, data can be [sent to] and
/// [received from] any other socket address.
///
/// This type is an async version of [`std::os::unix::net::UnixDatagram`].
///
/// [`std::os::unix::net::UnixDatagram`]:
/// https://doc.rust-lang.org/std/os/unix/net/struct.UnixDatagram.html
/// [`bind`]: #method.bind
/// [received from]: #method.recv_from
/// [sent to]: #method.send_to
///
/// ## Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::os::unix::net::UnixDatagram;
///
/// let socket = UnixDatagram::bind("/tmp/socket1").await?;
/// socket.send_to(b"hello world", "/tmp/socket2").await?;
///
/// let mut buf = vec![0u8; 1024];
/// let (n, peer) = socket.recv_from(&mut buf).await?;
/// #
/// # Ok(()) }) }
/// ```
pub struct UnixDatagram {
    #[cfg(not(feature = "docs.rs"))]
    io_handle: IoHandle<mio_uds::UnixDatagram>,

    raw_fd: RawFd,
}

impl UnixDatagram {
    #[cfg(not(feature = "docs.rs"))]
    fn new(socket: mio_uds::UnixDatagram) -> UnixDatagram {
        UnixDatagram {
            raw_fd: socket.as_raw_fd(),
            io_handle: IoHandle::new(socket),
        }
    }

    /// Creates a Unix datagram socket bound to the given path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::bind("/tmp/socket").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn bind<P: AsRef<Path>>(path: P) -> io::Result<UnixDatagram> {
        let path = path.as_ref().to_owned();
        let socket = blocking::spawn(async move { mio_uds::UnixDatagram::bind(path) }).await?;
        Ok(UnixDatagram::new(socket))
    }

    /// Creates a Unix datagram which is not bound to any address.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn unbound() -> io::Result<UnixDatagram> {
        let socket = mio_uds::UnixDatagram::unbound()?;
        Ok(UnixDatagram::new(socket))
    }

    /// Creates an unnamed pair of connected sockets.
    ///
    /// Returns two sockets which are connected to each other.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixDatagram;
    ///
    /// let (socket1, socket2) = UnixDatagram::pair()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn pair() -> io::Result<(UnixDatagram, UnixDatagram)> {
        let (a, b) = mio_uds::UnixDatagram::pair()?;
        let a = UnixDatagram::new(a);
        let b = UnixDatagram::new(b);
        Ok((a, b))
    }

    /// Connects the socket to the specified address.
    ///
    /// The [`send`] method may be used to send data to the specified address. [`recv`] and
    /// [`recv_from`] will only receive data from that address.
    ///
    /// [`send`]: #method.send
    /// [`recv`]: #method.recv
    /// [`recv_from`]: #method.recv_from
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.connect("/tmp/socket").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn connect<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        // TODO(stjepang): Connect the socket on a blocking pool.
        let p = path.as_ref();
        self.io_handle.get_ref().connect(p)
    }

    /// Returns the address of this socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::bind("/tmp/socket").await?;
    /// let addr = socket.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.io_handle.get_ref().local_addr()
    }

    /// Returns the address of this socket's peer.
    ///
    /// The [`connect`] method will connect the socket to a peer.
    ///
    /// [`connect`]: #method.connect
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixDatagram;
    ///
    /// let mut socket = UnixDatagram::unbound()?;
    /// socket.connect("/tmp/socket").await?;
    /// let peer = socket.peer_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.io_handle.get_ref().peer_addr()
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the address from where the data came.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixDatagram;
    ///
    /// let mut socket = UnixDatagram::unbound()?;
    /// let mut buf = vec![0; 1024];
    /// let (n, peer) = socket.recv_from(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        future::poll_fn(|cx| {
            ready!(self.io_handle.poll_readable(cx)?);

            match self.io_handle.get_ref().recv_from(buf) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.io_handle.clear_readable(cx)?;
                    Poll::Pending
                }
                Err(err) => Poll::Ready(Err(err)),
            }
        })
        .await
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::bind("/tmp/socket").await?;
    /// let mut buf = vec![0; 1024];
    /// let n = socket.recv(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        future::poll_fn(|cx| {
            ready!(self.io_handle.poll_writable(cx)?);

            match self.io_handle.get_ref().recv(buf) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.io_handle.clear_writable(cx)?;
                    Poll::Pending
                }
                Err(err) => Poll::Ready(Err(err)),
            }
        })
        .await
    }

    /// Sends data on the socket to the specified address.
    ///
    /// On success, returns the number of bytes written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixDatagram;
    ///
    /// let mut socket = UnixDatagram::unbound()?;
    /// socket.send_to(b"hello world", "/tmp/socket").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn send_to<P: AsRef<Path>>(&self, buf: &[u8], path: P) -> io::Result<usize> {
        future::poll_fn(|cx| {
            ready!(self.io_handle.poll_writable(cx)?);

            match self.io_handle.get_ref().send_to(buf, path.as_ref()) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.io_handle.clear_writable(cx)?;
                    Poll::Pending
                }
                Err(err) => Poll::Ready(Err(err)),
            }
        })
        .await
    }

    /// Sends data on the socket to the socket's peer.
    ///
    /// On success, returns the number of bytes written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixDatagram;
    ///
    /// let mut socket = UnixDatagram::unbound()?;
    /// socket.connect("/tmp/socket").await?;
    /// socket.send(b"hello world").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        future::poll_fn(|cx| {
            ready!(self.io_handle.poll_writable(cx)?);

            match self.io_handle.get_ref().send(buf) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(ref err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.io_handle.clear_writable(cx)?;
                    Poll::Pending
                }
                Err(err) => Poll::Ready(Err(err)),
            }
        })
        .await
    }

    /// Shut down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O calls on the specified portions to
    /// immediately return with an appropriate value (see the documentation of [`Shutdown`]).
    ///
    /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixDatagram;
    /// use std::net::Shutdown;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.shutdown(Shutdown::Both)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.io_handle.get_ref().shutdown(how)
    }
}

impl fmt::Debug for UnixDatagram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = f.debug_struct("UnixDatagram");
        builder.field("fd", &self.as_raw_fd());

        if let Ok(addr) = self.local_addr() {
            builder.field("local", &addr);
        }

        if let Ok(addr) = self.peer_addr() {
            builder.field("peer", &addr);
        }

        builder.finish()
    }
}

/// A Unix domain socket server, listening for connections.
///
/// After creating a `UnixListener` by [`bind`]ing it to a socket address, it listens for incoming
/// connections. These can be accepted by awaiting elements from the async stream of [`incoming`]
/// connections.
///
/// The socket will be closed when the value is dropped.
///
/// This type is an async version of [`std::os::unix::net::UnixListener`].
///
/// [`std::os::unix::net::UnixListener`]:
/// https://doc.rust-lang.org/std/os/unix/net/struct.UnixListener.html
/// [`bind`]: #method.bind
/// [`incoming`]: #method.incoming
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::os::unix::net::UnixListener;
/// use async_std::prelude::*;
///
/// let listener = UnixListener::bind("/tmp/socket").await?;
/// let mut incoming = listener.incoming();
///
/// while let Some(stream) = incoming.next().await {
///     let mut stream = stream?;
///     stream.write_all(b"hello world").await?;
/// }
/// #
/// # Ok(()) }) }
/// ```
pub struct UnixListener {
    #[cfg(not(feature = "docs.rs"))]
    io_handle: IoHandle<mio_uds::UnixListener>,

    raw_fd: RawFd,
}

impl UnixListener {
    /// Creates a Unix datagram listener bound to the given path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixListener;
    ///
    /// let listener = UnixListener::bind("/tmp/socket").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn bind<P: AsRef<Path>>(path: P) -> io::Result<UnixListener> {
        let path = path.as_ref().to_owned();
        let listener = blocking::spawn(async move { mio_uds::UnixListener::bind(path) }).await?;

        Ok(UnixListener {
            raw_fd: listener.as_raw_fd(),
            io_handle: IoHandle::new(listener),
        })
    }

    /// Accepts a new incoming connection to this listener.
    ///
    /// When a connection is established, the corresponding stream and address will be returned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixListener;
    ///
    /// let listener = UnixListener::bind("/tmp/socket").await?;
    /// let (socket, addr) = listener.accept().await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn accept(&self) -> io::Result<(UnixStream, SocketAddr)> {
        future::poll_fn(|cx| {
            ready!(self.io_handle.poll_readable(cx)?);

            match self.io_handle.get_ref().accept_std() {
                Ok(Some((io, addr))) => {
                    let mio_stream = mio_uds::UnixStream::from_stream(io)?;
                    let stream = UnixStream {
                        raw_fd: mio_stream.as_raw_fd(),
                        io_handle: IoHandle::new(mio_stream),
                    };
                    Poll::Ready(Ok((stream, addr)))
                }
                Ok(None) => {
                    self.io_handle.clear_readable(cx)?;
                    Poll::Pending
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
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixListener;
    /// use async_std::prelude::*;
    ///
    /// let listener = UnixListener::bind("/tmp/socket").await?;
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

    /// Returns the local socket address of this listener.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixListener;
    ///
    /// let listener = UnixListener::bind("/tmp/socket").await?;
    /// let addr = listener.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.io_handle.get_ref().local_addr()
    }
}

impl fmt::Debug for UnixListener {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = f.debug_struct("UnixListener");
        builder.field("fd", &self.as_raw_fd());

        if let Ok(addr) = self.local_addr() {
            builder.field("local", &addr);
        }

        builder.finish()
    }
}

/// A stream of incoming Unix domain socket connections.
///
/// This stream is infinite, i.e awaiting the next connection will never result in [`None`]. It is
/// created by the [`incoming`] method on [`UnixListener`].
///
/// This type is an async version of [`std::os::unix::net::Incoming`].
///
/// [`std::os::unix::net::Incoming`]: https://doc.rust-lang.org/std/os/unix/net/struct.Incoming.html
/// [`None`]: https://doc.rust-lang.org/std/option/enum.Option.html#variant.None
/// [`incoming`]: struct.UnixListener.html#method.incoming
/// [`UnixListener`]: struct.UnixListener.html
#[derive(Debug)]
pub struct Incoming<'a>(&'a UnixListener);

impl Stream for Incoming<'_> {
    type Item = io::Result<UnixStream>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let future = self.0.accept();
        futures::pin_mut!(future);

        let (socket, _) = ready!(future.poll(cx))?;
        Poll::Ready(Some(Ok(socket)))
    }
}

/// A Unix stream socket.
///
/// This type is an async version of [`std::os::unix::net::UnixStream`].
///
/// [`std::os::unix::net::UnixStream`]:
/// https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html
///
/// # Examples
///
/// ```no_run
/// # #![feature(async_await)]
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::os::unix::net::UnixStream;
/// use async_std::prelude::*;
///
/// let mut stream = UnixStream::connect("/tmp/socket").await?;
/// stream.write_all(b"hello world").await?;
///
/// let mut response = Vec::new();
/// stream.read_to_end(&mut response).await?;
/// #
/// # Ok(()) }) }
/// ```
pub struct UnixStream {
    #[cfg(not(feature = "docs.rs"))]
    io_handle: IoHandle<mio_uds::UnixStream>,

    raw_fd: RawFd,
}

impl UnixStream {
    /// Connects to the socket to the specified address.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixStream;
    ///
    /// let stream = UnixStream::connect("/tmp/socket").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn connect<P: AsRef<Path>>(path: P) -> io::Result<UnixStream> {
        enum State {
            Waiting(UnixStream),
            Error(io::Error),
            Done,
        }

        let path = path.as_ref().to_owned();
        let mut state = {
            match blocking::spawn(async move { mio_uds::UnixStream::connect(path) }).await {
                Ok(mio_stream) => State::Waiting(UnixStream {
                    raw_fd: mio_stream.as_raw_fd(),
                    io_handle: IoHandle::new(mio_stream),
                }),
                Err(err) => State::Error(err),
            }
        };

        future::poll_fn(|cx| {
            match &mut state {
                State::Waiting(stream) => {
                    ready!(stream.io_handle.poll_writable(cx)?);

                    if let Some(err) = stream.io_handle.get_ref().take_error()? {
                        return Poll::Ready(Err(err));
                    }
                }
                State::Error(_) => {
                    let err = match mem::replace(&mut state, State::Done) {
                        State::Error(err) => err,
                        _ => unreachable!(),
                    };

                    return Poll::Ready(Err(err));
                }
                State::Done => panic!("`UnixStream::connect()` future polled after completion"),
            }

            match mem::replace(&mut state, State::Done) {
                State::Waiting(stream) => Poll::Ready(Ok(stream)),
                _ => unreachable!(),
            }
        })
        .await
    }

    /// Creates an unnamed pair of connected sockets.
    ///
    /// Returns two streams which are connected to each other.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixStream;
    ///
    /// let stream = UnixStream::pair()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn pair() -> io::Result<(UnixStream, UnixStream)> {
        let (a, b) = mio_uds::UnixStream::pair()?;
        let a = UnixStream {
            raw_fd: a.as_raw_fd(),
            io_handle: IoHandle::new(a),
        };
        let b = UnixStream {
            raw_fd: b.as_raw_fd(),
            io_handle: IoHandle::new(b),
        };
        Ok((a, b))
    }

    /// Returns the socket address of the local half of this connection.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixStream;
    ///
    /// let stream = UnixStream::connect("/tmp/socket").await?;
    /// let addr = stream.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.io_handle.get_ref().local_addr()
    }

    /// Returns the socket address of the remote half of this connection.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixStream;
    ///
    /// let stream = UnixStream::connect("/tmp/socket").await?;
    /// let peer = stream.peer_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.io_handle.get_ref().peer_addr()
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O calls on the specified portions to
    /// immediately return with an appropriate value (see the documentation of [`Shutdown`]).
    ///
    /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
    ///
    /// ```no_run
    /// # #![feature(async_await)]
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixStream;
    /// use std::net::Shutdown;
    ///
    /// let stream = UnixStream::connect("/tmp/socket").await?;
    /// stream.shutdown(Shutdown::Both)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.io_handle.get_ref().shutdown(how)
    }
}

impl futures::io::AsyncRead for UnixStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_read(cx, buf)
    }
}

impl futures::io::AsyncRead for &UnixStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &self.io_handle).poll_read(cx, buf)
    }
}

impl futures::io::AsyncWrite for UnixStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &*self).poll_close(cx)
    }
}

impl futures::io::AsyncWrite for &UnixStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &self.io_handle).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &self.io_handle).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut &self.io_handle).poll_close(cx)
    }
}

impl fmt::Debug for UnixStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut builder = f.debug_struct("UnixStream");
        builder.field("fd", &self.as_raw_fd());

        if let Ok(addr) = self.local_addr() {
            builder.field("local", &addr);
        }

        if let Ok(addr) = self.peer_addr() {
            builder.field("peer", &addr);
        }

        builder.finish()
    }
}

#[cfg(unix)]
impl From<std::os::unix::net::UnixStream> for UnixStream {
    /// Converts a `std::os::unix::net::UnixStream` into its asynchronous equivalent.
    fn from(stream: std::os::unix::net::UnixStream) -> UnixStream {
        let mio_stream = mio_uds::UnixStream::from_stream(stream).unwrap();
        UnixStream {
            raw_fd: mio_stream.as_raw_fd(),
            io_handle: IoHandle::new(mio_stream),
        }
    }
}

#[cfg(unix)]
impl From<std::os::unix::net::UnixDatagram> for UnixDatagram {
    /// Converts a `std::os::unix::net::UnixDatagram` into its asynchronous equivalent.
    fn from(datagram: std::os::unix::net::UnixDatagram) -> UnixDatagram {
        let mio_datagram = mio_uds::UnixDatagram::from_datagram(datagram).unwrap();
        UnixDatagram {
            raw_fd: mio_datagram.as_raw_fd(),
            io_handle: IoHandle::new(mio_datagram),
        }
    }
}

#[cfg(unix)]
impl From<std::os::unix::net::UnixListener> for UnixListener {
    /// Converts a `std::os::unix::net::UnixListener` into its asynchronous equivalent.
    fn from(listener: std::os::unix::net::UnixListener) -> UnixListener {
        let mio_listener = mio_uds::UnixListener::from_listener(listener).unwrap();
        UnixListener {
            raw_fd: mio_listener.as_raw_fd(),
            io_handle: IoHandle::new(mio_listener),
        }
    }
}

impl AsRawFd for UnixListener {
    fn as_raw_fd(&self) -> RawFd {
        self.raw_fd
    }
}

impl FromRawFd for UnixListener {
    unsafe fn from_raw_fd(fd: RawFd) -> UnixListener {
        let listener = std::os::unix::net::UnixListener::from_raw_fd(fd);
        listener.into()
    }
}

impl IntoRawFd for UnixListener {
    fn into_raw_fd(self) -> RawFd {
        self.raw_fd
    }
}

impl AsRawFd for UnixStream {
    fn as_raw_fd(&self) -> RawFd {
        self.raw_fd
    }
}

impl FromRawFd for UnixStream {
    unsafe fn from_raw_fd(fd: RawFd) -> UnixStream {
        let stream = std::os::unix::net::UnixStream::from_raw_fd(fd);
        stream.into()
    }
}

impl IntoRawFd for UnixStream {
    fn into_raw_fd(self) -> RawFd {
        self.raw_fd
    }
}

impl AsRawFd for UnixDatagram {
    fn as_raw_fd(&self) -> RawFd {
        self.raw_fd
    }
}

impl FromRawFd for UnixDatagram {
    unsafe fn from_raw_fd(fd: RawFd) -> UnixDatagram {
        let datagram = std::os::unix::net::UnixDatagram::from_raw_fd(fd);
        datagram.into()
    }
}

impl IntoRawFd for UnixDatagram {
    fn into_raw_fd(self) -> RawFd {
        self.raw_fd
    }
}

cfg_if! {
    if #[cfg(feature = "docs.rs")] {
        /// An address associated with a Unix socket.
        ///
        /// # Examples
        ///
        /// ```
        /// use async_std::os::unix::net::UnixListener;
        ///
        /// let socket = UnixListener::bind("/tmp/socket").await?;
        /// let addr = socket.local_addr()?;
        /// ```
        #[derive(Clone)]
        pub struct SocketAddr {
            _private: (),
        }

        impl SocketAddr {
            /// Returns `true` if the address is unnamed.
            ///
            /// # Examples
            ///
            /// A named address:
            ///
            /// ```no_run
            /// use async_std::os::unix::net::UnixListener;
            ///
            /// let socket = UnixListener::bind("/tmp/socket").await?;
            /// let addr = socket.local_addr()?;
            /// assert_eq!(addr.is_unnamed(), false);
            /// ```
            ///
            /// An unnamed address:
            ///
            /// ```no_run
            /// use async_std::os::unix::net::UnixDatagram;
            ///
            /// let socket = UnixDatagram::unbound().await?;
            /// let addr = socket.local_addr()?;
            /// assert_eq!(addr.is_unnamed(), true);
            /// ```
            pub fn is_unnamed(&self) -> bool {
                unreachable!()
            }

            /// Returns the contents of this address if it is a `pathname` address.
            ///
            /// # Examples
            ///
            /// With a pathname:
            ///
            /// ```no_run
            /// use async_std::os::unix::net::UnixListener;
            /// use std::path::Path;
            ///
            /// let socket = UnixListener::bind("/tmp/socket").await?;
            /// let addr = socket.local_addr()?;
            /// assert_eq!(addr.as_pathname(), Some(Path::new("/tmp/socket")));
            /// ```
            ///
            /// Without a pathname:
            ///
            /// ```
            /// use async_std::os::unix::net::UnixDatagram;
            ///
            /// let socket = UnixDatagram::unbound()?;
            /// let addr = socket.local_addr()?;
            /// assert_eq!(addr.as_pathname(), None);
            /// ```
            pub fn as_pathname(&self) -> Option<&Path> {
                unreachable!()
            }
        }

        impl fmt::Debug for SocketAddr {
            fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                unreachable!()
            }
        }
    } else {
        #[doc(inline)]
        pub use std::os::unix::net::SocketAddr;
    }
}
