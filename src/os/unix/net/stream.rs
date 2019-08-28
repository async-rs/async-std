//! Unix-specific networking extensions.

use std::fmt;
use std::mem;
use std::net::Shutdown;
use std::path::Path;
use std::pin::Pin;

use futures::io::{AsyncRead, AsyncWrite};
use mio_uds;

use super::SocketAddr;
use crate::future;
use crate::io;
use crate::net::driver::IoHandle;
use crate::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
use crate::task::{blocking, Context, Poll};

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
    #[cfg(not(feature = "docs"))]
    pub(super) io_handle: IoHandle<mio_uds::UnixStream>,

    pub(super) raw_fd: RawFd,
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
                    futures::ready!(stream.io_handle.poll_writable(cx)?);

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

impl AsyncRead for UnixStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &*self).poll_read(cx, buf)
    }
}

impl AsyncRead for &UnixStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut &self.io_handle).poll_read(cx, buf)
    }
}

impl AsyncWrite for UnixStream {
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

impl AsyncWrite for &UnixStream {
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
