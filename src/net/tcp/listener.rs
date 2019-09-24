use std::net::SocketAddr;
use std::pin::Pin;

use cfg_if::cfg_if;

use super::TcpStream;
use crate::future::Future;
use crate::io;
use crate::net::ToSocketAddrs;
use crate::task::{Context, Poll};

cfg_if! {
    if #[cfg(not(target_os = "unknown"))] {
        use crate::future;
        use crate::net::driver::Watcher;
        use super::stream::Inner as TcpStreamInner;
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
    inner: Inner,
}

cfg_if! {
    if #[cfg(not(target_os = "unknown"))] {
        #[derive(Debug)]
        struct Inner {
            watcher: Watcher<mio::net::TcpListener>,
        }
    } else {
        #[derive(Debug)]
        struct Inner;
    }
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
        bind(addrs).await
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
        accept(self).await
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
        local_addr(self)
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

impl<'a> futures_core::stream::Stream for Incoming<'a> {
    type Item = io::Result<TcpStream>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let future = self.0.accept();
        pin_utils::pin_mut!(future);

        let (socket, _) = futures_core::ready!(future.poll(cx))?;
        Poll::Ready(Some(Ok(socket)))
    }
}

impl From<std::net::TcpListener> for TcpListener {
    /// Converts a `std::net::TcpListener` into its asynchronous equivalent.
    fn from(listener: std::net::TcpListener) -> TcpListener {
        from(listener)
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
                self.inner.watcher.get_ref().as_raw_fd()
            }
        }

        impl FromRawFd for TcpListener {
            unsafe fn from_raw_fd(fd: RawFd) -> TcpListener {
                std::net::TcpListener::from_raw_fd(fd).into()
            }
        }

        impl IntoRawFd for TcpListener {
            fn into_raw_fd(self) -> RawFd {
                self.inner.watcher.into_inner().into_raw_fd()
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

cfg_if! {
    if #[cfg(not(target_os = "unknown"))] {
        async fn bind<A: ToSocketAddrs>(addrs: A) -> io::Result<TcpListener> {
            let mut last_err = None;

            for addr in addrs.to_socket_addrs().await? {
                match mio::net::TcpListener::bind(&addr) {
                    Ok(mio_listener) => {
                        return Ok(TcpListener {
                            inner: Inner {
                                watcher: Watcher::new(mio_listener),
                            },
                        });
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

        async fn accept(listener: &TcpListener) -> io::Result<(TcpStream, SocketAddr)> {
            let (io, addr) =
                future::poll_fn(|cx| listener.inner.watcher.poll_read_with(cx, |inner| inner.accept_std()))
                    .await?;

            let mio_stream = mio::net::TcpStream::from_stream(io)?;
            let stream = TcpStream {
                inner: TcpStreamInner {
                    watcher: Watcher::new(mio_stream),
                },
            };
            Ok((stream, addr))
        }

        fn local_addr(listener: &TcpListener) -> io::Result<SocketAddr> {
            listener.inner.watcher.get_ref().local_addr()
        }

        fn from(listener: std::net::TcpListener) -> TcpListener {
            let mio_listener = mio::net::TcpListener::from_std(listener).unwrap();
            TcpListener {
                inner: Inner {
                    watcher: Watcher::new(mio_listener),
                },
            }
        }

    } else {
        async fn bind<A: ToSocketAddrs>(_: A) -> io::Result<TcpListener> {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "TCP sockets unsupported on this platform",
            ))
        }

        async fn accept(_: &TcpListener) -> io::Result<(TcpStream, SocketAddr)> {
            unreachable!()
        }

        fn local_addr(_: &TcpListener) -> io::Result<SocketAddr> {
            unreachable!()
        }

        fn from(_: std::net::TcpListener) -> TcpListener {
            // We can never successfully build a `std::net::TcpListener` on an unknown OS.
            unreachable!()
        }
    }
}
