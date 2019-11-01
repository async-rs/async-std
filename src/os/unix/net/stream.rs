//! Unix-specific networking extensions.

use std::fmt;
use std::io::{Read as _, Write as _};
use std::net::Shutdown;
use std::pin::Pin;

use mio_uds;

use super::SocketAddr;
use crate::io::{self, Read, Write};
use crate::net::driver::Watcher;
use crate::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use crate::path::Path;
use crate::task::{spawn_blocking, Context, Poll};

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
    pub(super) watcher: Watcher<mio_uds::UnixStream>,
}

impl UnixStream {
    /// Connects to the socket to the specified address.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::os::unix::net::UnixStream;
    ///
    /// let stream = UnixStream::connect("/tmp/socket").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn connect<P: AsRef<Path>>(path: P) -> io::Result<UnixStream> {
        let path = path.as_ref().to_owned();

        spawn_blocking(move || {
            let std_stream = std::os::unix::net::UnixStream::connect(path)?;
            let mio_stream = mio_uds::UnixStream::from_stream(std_stream)?;
            Ok(UnixStream {
                watcher: Watcher::new(mio_stream),
            })
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
            watcher: Watcher::new(a),
        };
        let b = UnixStream {
            watcher: Watcher::new(b),
        };
        Ok((a, b))
    }

    /// Returns the socket address of the local half of this connection.
    ///
    /// # Examples
    ///
    /// ```no_run
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
        self.watcher.get_ref().local_addr()
    }

    /// Returns the socket address of the remote half of this connection.
    ///
    /// # Examples
    ///
    /// ```no_run
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
        self.watcher.get_ref().peer_addr()
    }

    /// Shuts down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O calls on the specified portions to
    /// immediately return with an appropriate value (see the documentation of [`Shutdown`]).
    ///
    /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
    ///
    /// ```no_run
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
        self.watcher.get_ref().shutdown(how)
    }
}

impl Read for UnixStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_read(cx, buf)
    }
}

impl Read for &UnixStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        self.watcher.poll_read_with(cx, |mut inner| inner.read(buf))
    }
}

impl Write for UnixStream {
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

impl Write for &UnixStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.watcher
            .poll_write_with(cx, |mut inner| inner.write(buf))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.watcher.poll_write_with(cx, |mut inner| inner.flush())
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
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

impl From<std::os::unix::net::UnixStream> for UnixStream {
    /// Converts a `std::os::unix::net::UnixStream` into its asynchronous equivalent.
    fn from(stream: std::os::unix::net::UnixStream) -> UnixStream {
        let mio_stream = mio_uds::UnixStream::from_stream(stream).unwrap();
        UnixStream {
            watcher: Watcher::new(mio_stream),
        }
    }
}

impl AsRawFd for UnixStream {
    fn as_raw_fd(&self) -> RawFd {
        self.watcher.get_ref().as_raw_fd()
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
        self.watcher.into_inner().into_raw_fd()
    }
}
