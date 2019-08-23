use std::io;
use std::net::SocketAddr;

use cfg_if::cfg_if;
use futures::future;
use std::net::{Ipv4Addr, Ipv6Addr};

use crate::net::driver::IoHandle;
use crate::net::ToSocketAddrs;
use crate::task::Poll;

/// A UDP socket.
///
/// After creating a `UdpSocket` by [`bind`]ing it to a socket address, data can be [sent to] and
/// [received from] any other socket address.
///
/// As stated in the User Datagram Protocol's specification in [IETF RFC 768], UDP is an unordered,
/// unreliable protocol. Refer to [`TcpListener`] and [`TcpStream`] for async TCP primitives.
///
/// This type is an async version of [`std::net::UdpSocket`].
///
/// [`bind`]: #method.bind
/// [received from]: #method.recv_from
/// [sent to]: #method.send_to
/// [`TcpListener`]: struct.TcpListener.html
/// [`TcpStream`]: struct.TcpStream.html
/// [`std::net`]: https://doc.rust-lang.org/std/net/index.html
/// [IETF RFC 768]: https://tools.ietf.org/html/rfc768
/// [`std::net::UdpSocket`]: https://doc.rust-lang.org/std/net/struct.UdpSocket.html
///
/// ## Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
/// #
/// use async_std::net::UdpSocket;
///
/// let socket = UdpSocket::bind("127.0.0.1:8080").await?;
/// let mut buf = vec![0u8; 1024];
///
/// loop {
///     let (n, peer) = socket.recv_from(&mut buf).await?;
///     socket.send_to(&buf[..n], &peer).await?;
/// }
/// #
/// # }) }
/// ```
#[derive(Debug)]
pub struct UdpSocket {
    io_handle: IoHandle<mio::net::UdpSocket>,

    #[cfg(unix)]
    raw_fd: std::os::unix::io::RawFd,
    // #[cfg(windows)]
    // raw_socket: std::os::windows::io::RawSocket,
}

impl UdpSocket {
    /// Creates a UDP socket from the given address.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port to this socket. The
    /// port allocated can be queried via the [`local_addr`] method.
    ///
    /// [`local_addr`]: #method.local_addr
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<UdpSocket> {
        let mut last_err = None;

        for addr in addr.to_socket_addrs().await? {
            match mio::net::UdpSocket::bind(&addr) {
                Ok(mio_socket) => {
                    #[cfg(unix)]
                    let socket = UdpSocket {
                        raw_fd: mio_socket.as_raw_fd(),
                        io_handle: IoHandle::new(mio_socket),
                    };

                    #[cfg(windows)]
                    let socket = UdpSocket {
                        // raw_socket: mio_socket.as_raw_socket(),
                        io_handle: IoHandle::new(mio_socket),
                    };

                    return Ok(socket);
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

    /// Returns the local address that this listener is bound to.
    ///
    /// This can be useful, for example, when binding to port 0 to figure out which port was
    /// actually bound.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    ///	use async_std::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    /// let addr = socket.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.io_handle.get_ref().local_addr()
    }

    /// Sends data on the socket to the given address.
    ///
    /// On success, returns the number of bytes written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::UdpSocket;
    ///
    /// const THE_MERCHANT_OF_VENICE: &[u8] = b"
    ///     If you prick us, do we not bleed?
    ///     If you tickle us, do we not laugh?
    ///     If you poison us, do we not die?
    ///     And if you wrong us, shall we not revenge?
    /// ";
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    ///
    /// let addr = "127.0.0.1:7878";
    /// let sent = socket.send_to(THE_MERCHANT_OF_VENICE, &addr).await?;
    /// println!("Sent {} bytes to {}", sent, addr);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn send_to<A: ToSocketAddrs>(&self, buf: &[u8], addrs: A) -> io::Result<usize> {
        let addr = match addrs.to_socket_addrs().await?.next() {
            Some(addr) => addr,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "no addresses to send data to",
                ));
            }
        };

        future::poll_fn(|cx| {
            futures::ready!(self.io_handle.poll_writable(cx)?);

            match self.io_handle.get_ref().send_to(buf, &addr) {
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

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the origin.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    ///
    /// let mut buf = vec![0; 1024];
    /// let (n, peer) = socket.recv_from(&mut buf).await?;
    /// println!("Received {} bytes from {}", n, peer);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        future::poll_fn(|cx| {
            futures::ready!(self.io_handle.poll_readable(cx)?);

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

    /// Connects the UDP socket to a remote address.
    ///
    /// When connected, methods [`send`] and [`recv`] will use the specified address for sending
    /// and receiving messages. Additionally, a filter will be applied to [`recv_from`] so that it
    /// only receives messages from that same address.
    ///
    /// [`send`]: #method.send
    /// [`recv`]: #method.recv
    /// [`recv_from`]: #method.recv_from
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    ///	use async_std::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    /// socket.connect("127.0.0.1:8080").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn connect<A: ToSocketAddrs>(&self, addrs: A) -> io::Result<()> {
        let mut last_err = None;

        for addr in addrs.to_socket_addrs().await? {
            match self.io_handle.get_ref().connect(addr) {
                Ok(()) => return Ok(()),
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

    /// Sends data on the socket to the given address.
    ///
    /// On success, returns the number of bytes written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::UdpSocket;
    ///
    /// const THE_MERCHANT_OF_VENICE: &[u8] = b"
    ///     If you prick us, do we not bleed?
    ///     If you tickle us, do we not laugh?
    ///     If you poison us, do we not die?
    ///     And if you wrong us, shall we not revenge?
    /// ";
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    ///
    /// let addr = "127.0.0.1:7878";
    /// let sent = socket.send_to(THE_MERCHANT_OF_VENICE, &addr).await?;
    /// println!("Sent {} bytes to {}", sent, addr);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        future::poll_fn(|cx| {
            futures::ready!(self.io_handle.poll_writable(cx)?);

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

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the origin.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use async_std::net::UdpSocket;
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    ///
    /// let mut buf = vec![0; 1024];
    /// let (n, peer) = socket.recv_from(&mut buf).await?;
    /// println!("Received {} bytes from {}", n, peer);
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        future::poll_fn(|cx| {
            futures::ready!(self.io_handle.poll_readable(cx)?);

            match self.io_handle.get_ref().recv(buf) {
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

    /// Gets the value of the `SO_BROADCAST` option for this socket.
    ///
    /// For more information about this option, see [`set_broadcast`].
    ///
    /// [`set_broadcast`]: #method.set_broadcast
    pub fn broadcast(&self) -> io::Result<bool> {
        self.io_handle.get_ref().broadcast()
    }

    /// Sets the value of the `SO_BROADCAST` option for this socket.
    ///
    /// When enabled, this socket is allowed to send packets to a broadcast address.
    pub fn set_broadcast(&self, on: bool) -> io::Result<()> {
        self.io_handle.get_ref().set_broadcast(on)
    }

    /// Gets the value of the `IP_MULTICAST_LOOP` option for this socket.
    ///
    /// For more information about this option, see [`set_multicast_loop_v4`].
    ///
    /// [`set_multicast_loop_v4`]: #method.set_multicast_loop_v4
    pub fn multicast_loop_v4(&self) -> io::Result<bool> {
        self.io_handle.get_ref().multicast_loop_v4()
    }

    /// Sets the value of the `IP_MULTICAST_LOOP` option for this socket.
    ///
    /// If enabled, multicast packets will be looped back to the local socket.
    ///
    /// # Note
    ///
    /// This may not have any affect on IPv6 sockets.
    pub fn set_multicast_loop_v4(&self, on: bool) -> io::Result<()> {
        self.io_handle.get_ref().set_multicast_loop_v4(on)
    }

    /// Gets the value of the `IP_MULTICAST_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_multicast_ttl_v4`].
    ///
    /// [`set_multicast_ttl_v4`]: #method.set_multicast_ttl_v4
    pub fn multicast_ttl_v4(&self) -> io::Result<u32> {
        self.io_handle.get_ref().multicast_ttl_v4()
    }

    /// Sets the value of the `IP_MULTICAST_TTL` option for this socket.
    ///
    /// Indicates the time-to-live value of outgoing multicast packets for this socket. The default
    /// value is 1 which means that multicast packets don't leave the local network unless
    /// explicitly requested.
    ///
    /// # Note
    ///
    /// This may not have any affect on IPv6 sockets.
    pub fn set_multicast_ttl_v4(&self, ttl: u32) -> io::Result<()> {
        self.io_handle.get_ref().set_multicast_ttl_v4(ttl)
    }

    /// Gets the value of the `IPV6_MULTICAST_LOOP` option for this socket.
    ///
    /// For more information about this option, see [`set_multicast_loop_v6`].
    ///
    /// [`set_multicast_loop_v6`]: #method.set_multicast_loop_v6
    pub fn multicast_loop_v6(&self) -> io::Result<bool> {
        self.io_handle.get_ref().multicast_loop_v6()
    }

    /// Sets the value of the `IPV6_MULTICAST_LOOP` option for this socket.
    ///
    /// Controls whether this socket sees the multicast packets it sends itself.
    ///
    /// # Note
    ///
    /// This may not have any affect on IPv4 sockets.
    pub fn set_multicast_loop_v6(&self, on: bool) -> io::Result<()> {
        self.io_handle.get_ref().set_multicast_loop_v6(on)
    }

    /// Gets the value of the `IP_TTL` option for this socket.
    ///
    /// For more information about this option, see [`set_ttl`].
    ///
    /// [`set_ttl`]: #method.set_ttl
    pub fn ttl(&self) -> io::Result<u32> {
        self.io_handle.get_ref().ttl()
    }

    /// Sets the value for the `IP_TTL` option on this socket.
    ///
    /// This value sets the time-to-live field that is used in every packet sent
    /// from this socket.
    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.io_handle.get_ref().set_ttl(ttl)
    }

    /// Executes an operation of the `IP_ADD_MEMBERSHIP` type.
    ///
    /// This method specifies a new multicast group for this socket to join. The address must be
    /// a valid multicast address, and `interface` is the address of the local interface with which
    /// the system should join the multicast group. If it's equal to `INADDR_ANY` then an
    /// appropriate interface is chosen by the system.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use std::net::Ipv4Addr;
    ///
    /// use async_std::net::UdpSocket;
    ///
    /// let interface = Ipv4Addr::new(0, 0, 0, 0);
    /// let mdns_addr = Ipv4Addr::new(224, 0, 0, 123);
    ///
    /// let socket = UdpSocket::bind("127.0.0.1:0").await?;
    /// socket.join_multicast_v4(&mdns_addr, &interface)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn join_multicast_v4(&self, multiaddr: &Ipv4Addr, interface: &Ipv4Addr) -> io::Result<()> {
        self.io_handle
            .get_ref()
            .join_multicast_v4(multiaddr, interface)
    }

    /// Executes an operation of the `IPV6_ADD_MEMBERSHIP` type.
    ///
    /// This method specifies a new multicast group for this socket to join. The address must be
    /// a valid multicast address, and `interface` is the index of the interface to join/leave (or
    /// 0 to indicate any interface).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
    /// #
    /// use std::net::{Ipv6Addr, SocketAddr};
    ///
    /// use async_std::net::UdpSocket;
    ///
    /// let socket_addr = SocketAddr::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0).into(), 0);
    /// let mdns_addr = Ipv6Addr::new(0xFF02, 0, 0, 0, 0, 0, 0, 0x0123) ;
    /// let socket = UdpSocket::bind(&socket_addr).await?;
    ///
    /// socket.join_multicast_v6(&mdns_addr, 0)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn join_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> io::Result<()> {
        self.io_handle
            .get_ref()
            .join_multicast_v6(multiaddr, interface)
    }

    /// Executes an operation of the `IP_DROP_MEMBERSHIP` type.
    ///
    /// For more information about this option, see [`join_multicast_v4`].
    ///
    /// [`join_multicast_v4`]: #method.join_multicast_v4
    pub fn leave_multicast_v4(&self, multiaddr: &Ipv4Addr, interface: &Ipv4Addr) -> io::Result<()> {
        self.io_handle
            .get_ref()
            .leave_multicast_v4(multiaddr, interface)
    }

    /// Executes an operation of the `IPV6_DROP_MEMBERSHIP` type.
    ///
    /// For more information about this option, see [`join_multicast_v6`].
    ///
    /// [`join_multicast_v6`]: #method.join_multicast_v6
    pub fn leave_multicast_v6(&self, multiaddr: &Ipv6Addr, interface: u32) -> io::Result<()> {
        self.io_handle
            .get_ref()
            .leave_multicast_v6(multiaddr, interface)
    }
}

impl From<std::net::UdpSocket> for UdpSocket {
    /// Converts a `std::net::UdpSocket` into its asynchronous equivalent.
    fn from(socket: std::net::UdpSocket) -> UdpSocket {
        let mio_socket = mio::net::UdpSocket::from_socket(socket).unwrap();

        #[cfg(unix)]
        let socket = UdpSocket {
            raw_fd: mio_socket.as_raw_fd(),
            io_handle: IoHandle::new(mio_socket),
        };

        #[cfg(windows)]
        let socket = UdpSocket {
            // raw_socket: mio_socket.as_raw_socket(),
            io_handle: IoHandle::new(mio_socket),
        };

        socket
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
        impl AsRawFd for UdpSocket {
            fn as_raw_fd(&self) -> RawFd {
                self.raw_fd
            }
        }

        impl FromRawFd for UdpSocket {
            unsafe fn from_raw_fd(fd: RawFd) -> UdpSocket {
                std::net::UdpSocket::from_raw_fd(fd).into()
            }
        }

        impl IntoRawFd for UdpSocket {
            fn into_raw_fd(self) -> RawFd {
                self.raw_fd
            }
        }
    }
}

#[cfg_attr(feature = "docs", doc(cfg(windows)))]
cfg_if! {
    if #[cfg(any(windows, feature = "docs"))] {
        // use std::os::windows::io::{AsRawSocket, FromRawSocket, IntoRawSocket, RawSocket};
        //
        // impl AsRawSocket for UdpSocket {
        //     fn as_raw_socket(&self) -> RawSocket {
        //         self.raw_socket
        //     }
        // }
        //
        // impl FromRawSocket for UdpSocket {
        //     unsafe fn from_raw_socket(handle: RawSocket) -> UdpSocket {
        //         net::UdpSocket::from_raw_socket(handle).into()
        //     }
        // }
        //
        // impl IntoRawSocket for UdpSocket {
        //     fn into_raw_socket(self) -> RawSocket {
        //         self.raw_socket
        //     }
        // }
    }
}
