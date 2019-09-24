//! Networking primitives for TCP/UDP communication.
//!
//! For OS-specific networking primitives like Unix domain sockets, refer to the [`async_std::os`]
//! module.
//!
//! This module is an async version of [`std::net`].
//!
//! [`async_std::os`]: ../os/index.html
//! [`std::net`]: https://doc.rust-lang.org/std/net/index.html
//!
//! ## Examples
//!
//! A simple UDP echo server:
//!
//! ```no_run
//! # fn main() -> std::io::Result<()> { async_std::task::block_on(async {
//! #
//! use async_std::net::UdpSocket;
//!
//! let socket = UdpSocket::bind("127.0.0.1:8080").await?;
//! let mut buf = vec![0u8; 1024];
//!
//! loop {
//!     let (n, peer) = socket.recv_from(&mut buf).await?;
//!     socket.send_to(&buf[..n], &peer).await?;
//! }
//! #
//! # }) }
//! ```

pub use addr::ToSocketAddrs;
pub use tcp::{Incoming, TcpListener, TcpStream};
pub use udp::UdpSocket;

mod addr;
pub(crate) mod driver;
mod tcp;
mod udp;
