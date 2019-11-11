//! Unix-specific networking extensions.

use std::fmt;
use std::pin::Pin;
use std::future::Future;

use mio_uds;

use super::SocketAddr;
use super::UnixStream;
use crate::future;
use crate::io;
use crate::net::driver::Watcher;
use crate::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use crate::path::Path;
use crate::stream::Stream;
use crate::task::{spawn_blocking, Context, Poll};

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
    watcher: Watcher<mio_uds::UnixListener>,
}

impl UnixListener {
    /// Creates a Unix datagram listener bound to the given path.
    ///
    /// # Examples
    ///
    /// ```no_run
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
        let listener = spawn_blocking(move || mio_uds::UnixListener::bind(path)).await?;

        Ok(UnixListener {
            watcher: Watcher::new(listener),
        })
    }

    /// Accepts a new incoming connection to this listener.
    ///
    /// When a connection is established, the corresponding stream and address will be returned.
    ///
    /// # Examples
    ///
    /// ```no_run
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
            let res = futures_core::ready!(self.watcher.poll_read_with(cx, |inner| {
                match inner.accept_std() {
                    // Converting to `WouldBlock` so that the watcher will
                    // add the waker of this task to a list of readers.
                    Ok(None) => Err(io::ErrorKind::WouldBlock.into()),
                    res => res,
                }
            }));

            match res? {
                Some((io, addr)) => {
                    let mio_stream = mio_uds::UnixStream::from_stream(io)?;
                    let stream = UnixStream {
                        watcher: Watcher::new(mio_stream),
                    };
                    Poll::Ready(Ok((stream, addr)))
                }
                // This should never happen since `None` is converted to `WouldBlock`
                None => unreachable!(),
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
        self.watcher.get_ref().local_addr()
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
        pin_utils::pin_mut!(future);

        let (socket, _) = futures_core::ready!(future.poll(cx))?;
        Poll::Ready(Some(Ok(socket)))
    }
}

impl From<std::os::unix::net::UnixListener> for UnixListener {
    /// Converts a `std::os::unix::net::UnixListener` into its asynchronous equivalent.
    fn from(listener: std::os::unix::net::UnixListener) -> UnixListener {
        let mio_listener = mio_uds::UnixListener::from_listener(listener).unwrap();
        UnixListener {
            watcher: Watcher::new(mio_listener),
        }
    }
}

impl AsRawFd for UnixListener {
    fn as_raw_fd(&self) -> RawFd {
        self.watcher.get_ref().as_raw_fd()
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
        self.watcher.into_inner().into_raw_fd()
    }
}
