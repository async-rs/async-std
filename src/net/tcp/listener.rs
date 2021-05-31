use std::fmt;
use std::net::SocketAddr;
use std::net::TcpStream as StdTcpStream;
use std::pin::Pin;

use async_io::Async;

use crate::io;
use crate::net::{TcpStream, ToSocketAddrs};
use crate::stream::Stream;
use crate::sync::Arc;
use crate::task::{ready, Context, Poll};

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
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::io;
/// use async_std::net::TcpListener;
/// use async_std::prelude::*;
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
    watcher: Async<std::net::TcpListener>,
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
        let addrs = addrs.to_socket_addrs().await?;

        for addr in addrs {
            match Async::<std::net::TcpListener>::bind(addr) {
                Ok(listener) => {
                    return Ok(TcpListener { watcher: listener });
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
        let (stream, addr) = self.watcher.accept().await?;
        let stream = TcpStream {
            watcher: Arc::new(stream),
        };
        Ok((stream, addr))
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
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::TcpListener;
    /// use async_std::prelude::*;
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
    #[must_use]
    pub fn incoming(&self) -> Incoming<'_> {
        Incoming {
            incoming: Box::pin(self.watcher.incoming()),
        }
    }

    /// Returns the local address that this listener is bound to.
    ///
    /// This can be useful, for example, to identify when binding to port 0 which port was assigned
    /// by the OS.
    ///
    /// # Examples
    ///
    /// ```no_run
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
        self.watcher.get_ref().local_addr()
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
pub struct Incoming<'a> {
    incoming: Pin<Box<dyn Stream<Item = io::Result<Async<StdTcpStream>>> + Send + Sync + 'a>>,
}

impl Stream for Incoming<'_> {
    type Item = io::Result<TcpStream>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let res = ready!(Pin::new(&mut self.incoming).poll_next(cx));
        Poll::Ready(res.map(|res| res.map(|stream| TcpStream { watcher: Arc::new(stream) })))
    }
}

impl fmt::Debug for Incoming<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Incoming {{ ... }}")
    }
}

impl From<std::net::TcpListener> for TcpListener {
    /// Converts a `std::net::TcpListener` into its asynchronous equivalent.
    fn from(listener: std::net::TcpListener) -> TcpListener {
        TcpListener {
            watcher: Async::new(listener).expect("TcpListener is known to be good"),
        }
    }
}

cfg_unix! {
    use crate::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

    impl AsRawFd for TcpListener {
        fn as_raw_fd(&self) -> RawFd {
            self.watcher.get_ref().as_raw_fd()
        }
    }

    impl FromRawFd for TcpListener {
        unsafe fn from_raw_fd(fd: RawFd) -> TcpListener {
            std::net::TcpListener::from_raw_fd(fd).into()
        }
    }

    impl IntoRawFd for TcpListener {
        fn into_raw_fd(self) -> RawFd {
            self.watcher.into_inner().unwrap().into_raw_fd()
        }
    }
}

cfg_windows! {
    use crate::os::windows::io::{
        AsRawSocket, FromRawSocket, IntoRawSocket, RawSocket,
    };

    impl AsRawSocket for TcpListener {
        fn as_raw_socket(&self) -> RawSocket {
            self.watcher.as_raw_socket()
        }
    }

    impl FromRawSocket for TcpListener {
        unsafe fn from_raw_socket(handle: RawSocket) -> TcpListener {
            std::net::TcpListener::from_raw_socket(handle).into()
        }
    }

    impl IntoRawSocket for TcpListener {
        fn into_raw_socket(self) -> RawSocket {
            self.watcher.into_inner().unwrap().into_raw_socket()
        }
    }
}
