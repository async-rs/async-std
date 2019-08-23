use std::net::SocketAddr;
use std::pin::Pin;

use cfg_if::cfg_if;
use futures::future;

use super::TcpStream;
use crate::future::Future;
use crate::io;
use crate::net::driver::IoHandle;
use crate::net::ToSocketAddrs;
use crate::task::{Context, Poll};

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

        for addr in addrs.to_socket_addrs().await? {
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
            futures::ready!(self.io_handle.poll_readable(cx)?);

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

        let (socket, _) = futures::ready!(future.poll(cx))?;
        Poll::Ready(Some(Ok(socket)))
    }
}

impl From<std::net::TcpListener> for TcpListener {
    /// Converts a `std::net::TcpListener` into its asynchronous equivalent.
    fn from(listener: std::net::TcpListener) -> TcpListener {
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
        impl AsRawFd for TcpListener {
            fn as_raw_fd(&self) -> RawFd {
                self.raw_fd
            }
        }

        impl FromRawFd for TcpListener {
            unsafe fn from_raw_fd(fd: RawFd) -> TcpListener {
                std::net::TcpListener::from_raw_fd(fd).into()
            }
        }

        impl IntoRawFd for TcpListener {
            fn into_raw_fd(self) -> RawFd {
                self.raw_fd
            }
        }
    }
}

#[cfg_attr(feature = "docs", doc(cfg(windows)))]
cfg_if! {
    if #[cfg(any(windows, feature = "docs"))] {
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
    }
}
